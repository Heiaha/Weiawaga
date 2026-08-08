use super::bitboard::*;
use super::moov::*;
use super::piece::*;
use super::square::*;
use super::traits::*;

use wide::*;

#[repr(C, align(64))]
struct Aligned<const N: usize>([u8; N]);

static NETWORK: Aligned<{ Network::NET_BYTES }> = Aligned(*include_bytes!("network.bin"));

struct Cursor {
    bytes: &'static [u8],
}

impl Cursor {
    fn take<T: bytemuck::Pod>(&mut self) -> &'static T {
        let (head, tail) = self.bytes.split_at(core::mem::size_of::<T>());
        self.bytes = tail;
        bytemuck::from_bytes(head)
    }
}

type Embedding = [i16x32; Network::L1 / Network::LANES];

fn clipped_relu(x: i16x32) -> i16x32 {
    x.max(i16x32::ZERO)
        .min(i16x32::splat(Network::INPUT_SCALE as i16))
}

#[derive(Clone)]
struct Embeddings {
    weights: &'static [Embedding; Network::N_INPUTS],
}

#[derive(Clone)]
struct Linear<const IN: usize, const OUT: usize> {
    weights: &'static [i16x32; IN],
    biases: &'static [i16; OUT],
}

impl<const IN: usize, const OUT: usize> Linear<IN, OUT> {
    // Fused clipped-relu-square dot product over both perspective halves,
    // side to move first. Returns the raw quantized sum; scaling and the
    // bias are the caller's concern.
    fn forward(&self, stm: &[i16x32], nstm: &[i16x32]) -> i32 {
        let (stm_weights, nstm_weights) = self.weights.split_at(stm.len());

        let dot = |acts: &[i16x32], weights: &[i16x32]| -> i32x16 {
            acts.iter()
                .zip(weights)
                .map(|(&act, &w)| {
                    let clamped = clipped_relu(act);
                    (w * clamped).dot(clamped)
                })
                .sum()
        };

        (dot(stm, stm_weights) + dot(nstm, nstm_weights)).reduce_add()
    }
}

#[derive(Clone)]
struct WdlLayer<const IN: usize> {
    weights: &'static [[f32; IN]; 3],
    biases: &'static [f32; 3],
}

impl<const IN: usize> WdlLayer<IN> {
    // Softmaxed loss/draw/win probabilities for the given activations.
    fn forward(&self, activations: &[f32; IN]) -> [f32; 3] {
        let mut logits = *self.biases;
        for (logit, weights) in logits.iter_mut().zip(self.weights) {
            *logit += weights
                .iter()
                .zip(activations)
                .map(|(w, a)| w * a)
                .sum::<f32>();
        }

        let max = logits.into_iter().fold(f32::MIN, f32::max);
        let exps = logits.map(|logit| (logit - max).exp());
        let sum = exps.iter().sum::<f32>();
        exps.map(|exp| exp / sum)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Perspective {
    bucket: usize,
    mirrored: bool,
}

impl Perspective {
    fn new(ksq_rel: SQ) -> Self {
        let mirrored = ksq_rel.file() >= File::E;
        let ksq_norm = if mirrored { ksq_rel.hmirror() } else { ksq_rel };
        Self {
            bucket: Self::king_bucket(ksq_norm),
            mirrored,
        }
    }

    fn king_bucket(ksq_norm: SQ) -> usize {
        match ksq_norm.rank() {
            Rank::One => 0,
            Rank::Two => 1,
            Rank::Three | Rank::Four => 2,
            _ => 3,
        }
    }

    fn feature_idx(self, pc: Piece, sq: SQ, color: Color) -> usize {
        let sq_rel = sq.relative(color);
        let sq_rel = if self.mirrored {
            sq_rel.hmirror()
        } else {
            sq_rel
        };
        pc.relative(color).index() * SQ::COUNT + sq_rel.index()
    }
}

#[derive(Clone, Copy)]
struct Delta {
    m: Move,
    pc: Piece,
    captured_pc: Option<Piece>,
}

#[derive(Clone, Copy, Default)]
struct Frame {
    perspectives: ColorMap<Perspective>,
    computed: ColorMap<bool>,
    delta: Option<Delta>,
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
struct Accumulator(Embedding);

impl Accumulator {
    fn update<const SIGN: i16>(&mut self, emb: &Embedding) {
        self.0
            .iter_mut()
            .zip(emb)
            .for_each(|(act, &w)| *act += SIGN * w);
    }

    fn move_piece(&mut self, from_emb: &Embedding, to_emb: &Embedding) {
        self.0
            .iter_mut()
            .zip(to_emb.iter().zip(from_emb))
            .for_each(|(act, (&w_to, &w_from))| *act += w_to - w_from);
    }

    // Out-of-place move_piece: the copy from the parent accumulator rides
    // along in the same pass.
    fn move_piece_from(&mut self, src: &Accumulator, from_emb: &Embedding, to_emb: &Embedding) {
        self.0
            .iter_mut()
            .zip(src.0.iter().zip(to_emb.iter().zip(from_emb)))
            .for_each(|(act, (&s, (&w_to, &w_from)))| *act = s + w_to - w_from);
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
struct Snapshot {
    acc: Accumulator,
    pieces: PieceMap<Bitboard>,
}

#[derive(Clone)]
pub struct Network {
    embeddings: [Embeddings; Self::N_KING_BUCKETS],
    hidden_layers: [Linear<{ 2 * Self::L1 / Self::LANES }, 1>; Self::N_BUCKETS],
    wdl_layers: [WdlLayer<{ 2 * Self::L1 }>; Self::N_BUCKETS],

    stack: Vec<ColorMap<Accumulator>>,
    frames: Vec<Frame>,
    idx: usize,
    snapshots: ColorMap<[[Snapshot; 2]; Self::N_KING_BUCKETS]>,
}

impl Network {
    pub fn new() -> Self {
        let bytes: &'static [u8] = &NETWORK.0;
        let (weights, biases) = bytes.split_at(2 * Self::W_I16);
        let mut w = Cursor { bytes: weights };
        let mut b = Cursor { bytes: biases };

        let embeddings = core::array::from_fn(|_| Embeddings { weights: w.take() });
        let input_bias = w.take::<Embedding>();

        let hidden_layers = core::array::from_fn(|_| Linear {
            weights: w.take(),
            biases: b.take(),
        });

        let wdl_weights: [_; Self::N_BUCKETS] = core::array::from_fn(|_| b.take());
        let wdl_biases: [_; Self::N_BUCKETS] = core::array::from_fn(|_| b.take());
        let wdl_layers = core::array::from_fn(|i| WdlLayer {
            weights: wdl_weights[i],
            biases: wdl_biases[i],
        });

        let cold = Snapshot {
            acc: Accumulator(*input_bias),
            pieces: PieceMap::default(),
        };

        Self {
            embeddings,
            hidden_layers,
            wdl_layers,
            stack: vec![
                ColorMap::new([Accumulator(*input_bias); Color::COUNT]);
                Self::N_ACCUMULATORS
            ],
            frames: vec![Frame::default(); Self::N_ACCUMULATORS],
            idx: 0,
            snapshots: ColorMap::new([[[cold; 2]; Self::N_KING_BUCKETS]; Color::COUNT]),
        }
    }

    #[inline]
    pub fn push(&mut self, m: Move, pc: Piece, captured: Option<Piece>) {
        debug_assert!(self.idx + 1 < Self::N_ACCUMULATORS);
        let mut perspectives = self.frames[self.idx].perspectives;
        if pc.type_of() == PieceType::King {
            let color = pc.color_of();
            perspectives[color] = Perspective::new(m.to_sq().relative(color));
        }

        self.idx += 1;
        self.frames[self.idx] = Frame {
            perspectives,
            computed: ColorMap::default(),
            delta: Some(Delta {
                m,
                pc,
                captured_pc: captured,
            }),
        };
    }

    #[inline]
    pub fn pop(&mut self) {
        debug_assert!(self.idx > 0);
        self.idx -= 1;
    }

    pub fn reset_perspective(&mut self, color: Color, ksq_rel: SQ, pieces: &PieceMap<Bitboard>) {
        self.frames[self.idx].perspectives[color] = Perspective::new(ksq_rel);
        self.refresh_from_snapshot(color, pieces);
    }

    fn refresh_from_snapshot(&mut self, color: Color, pieces: &PieceMap<Bitboard>) {
        let perspective = self.frames[self.idx].perspectives[color];
        let embedding = &self.embeddings[perspective.bucket];
        let snapshot =
            &mut self.snapshots[color][perspective.bucket][perspective.mirrored as usize];
        let emb = |pc: Piece, sq: SQ| &embedding.weights[perspective.feature_idx(pc, sq, color)];

        for pc in Piece::iter() {
            for sq in pieces[pc] & !snapshot.pieces[pc] {
                snapshot.acc.update::<1>(emb(pc, sq));
            }
            for sq in snapshot.pieces[pc] & !pieces[pc] {
                snapshot.acc.update::<-1>(emb(pc, sq));
            }
            snapshot.pieces[pc] = pieces[pc];
        }

        self.stack[self.idx][color] = snapshot.acc;
        self.frames[self.idx].computed[color] = true;
    }

    fn materialize_perspective(&mut self, color: Color, pieces: &PieceMap<Bitboard>) {
        if self.frames[self.idx].computed[color] {
            return;
        }

        let perspective = self.frames[self.idx].perspectives[color];

        // Walk to the nearest computed ancestor. A perspective change in
        // the span means incompatible feature bases.
        // rebuild from the snapshot instead.
        let mut idx = self.idx;
        loop {
            if self.frames[idx].perspectives[color] != perspective {
                self.refresh_from_snapshot(color, pieces);
                return;
            }
            if self.frames[idx].computed[color] {
                break;
            }
            debug_assert!(idx > 0, "Root accumulator must always be computed.");
            idx -= 1;
        }

        for i in idx + 1..=self.idx {
            self.materialize_ply(i, color);
        }
    }

    fn materialize_ply(&mut self, i: usize, color: Color) {
        debug_assert!(i > 0, "The root has no parent to materialize from.");
        let frame = self.frames[i];
        let Delta { m, pc, captured_pc } = frame
            .delta
            .expect("Uncomputed non-root ply must carry a delta.");
        let perspective = frame.perspectives[color];
        let embedding = &self.embeddings[perspective.bucket];
        let [parent, child] = self.stack.get_disjoint_mut([i - 1, i]).unwrap();
        let acc = &mut child[color];

        let (from_sq, to_sq) = m.squares();
        let pc_color = pc.color_of();
        let emb = |pc: Piece, sq: SQ| &embedding.weights[perspective.feature_idx(pc, sq, color)];

        let promo_pc = m
            .promotion()
            .map_or(pc, |pt| Piece::make_piece(pc_color, pt));
        acc.move_piece_from(&parent[color], emb(pc, from_sq), emb(promo_pc, to_sq));

        match m.flags() {
            MoveFlags::OO => {
                let rook = Piece::make_piece(pc_color, PieceType::Rook);
                acc.move_piece(
                    emb(rook, SQ::H1.relative(pc_color)),
                    emb(rook, SQ::F1.relative(pc_color)),
                );
            }
            MoveFlags::OOO => {
                let rook = Piece::make_piece(pc_color, PieceType::Rook);
                acc.move_piece(
                    emb(rook, SQ::A1.relative(pc_color)),
                    emb(rook, SQ::D1.relative(pc_color)),
                );
            }
            MoveFlags::EnPassant => {
                let captured_pc = Piece::make_piece(!pc_color, PieceType::Pawn);
                let captured_sq = to_sq + Direction::South.relative(pc_color);
                acc.update::<-1>(emb(captured_pc, captured_sq));
            }
            _ => {
                if let Some(captured_pc) = captured_pc {
                    acc.update::<-1>(emb(captured_pc, to_sq));
                }
            }
        }

        self.frames[i].computed[color] = true;
    }

    fn materialize(&mut self, pieces: &PieceMap<Bitboard>) {
        self.materialize_perspective(Color::White, pieces);
        self.materialize_perspective(Color::Black, pieces);
    }

    fn output_bucket(pieces: &PieceMap<Bitboard>) -> usize {
        let pop_count: usize = pieces.iter().map(|bb| usize::from(bb.pop_count())).sum();
        (pop_count - 2) / Self::BUCKET_DIV
    }

    pub fn eval(&mut self, ctm: Color, pieces: &PieceMap<Bitboard>) -> i32 {
        self.materialize(pieces);

        let acc = &self.stack[self.idx];
        let hidden_layer = &self.hidden_layers[Self::output_bucket(pieces)];

        let output = hidden_layer.forward(&acc[ctm].0, &acc[!ctm].0);

        i32::from(hidden_layer.biases[0]) * Self::NNUE2SCORE / Self::HIDDEN_SCALE
            + (output / Self::INPUT_SCALE) * Self::NNUE2SCORE / Self::COMB_SCALE
    }

    pub fn wdl(&mut self, ctm: Color, pieces: &PieceMap<Bitboard>) -> [f32; 3] {
        self.materialize(pieces);

        let acc = &self.stack[self.idx];

        let mut activations = [0.0; 2 * Self::L1];
        let lanes = [ctm, !ctm]
            .into_iter()
            .flat_map(|color| &acc[color].0)
            .flat_map(|&act| clipped_relu(act).to_array());
        for (activation, x) in activations.iter_mut().zip(lanes) {
            let a = f32::from(x) / Self::INPUT_SCALE as f32;
            *activation = a * a;
        }

        self.wdl_layers[Self::output_bucket(pieces)].forward(&activations)
    }
}

impl Network {
    const N_INPUTS: usize = Piece::COUNT * SQ::COUNT;
    const N_KING_BUCKETS: usize = 4;
    const N_ACCUMULATORS: usize = 1024;
    const L1: usize = 768;
    const N_BUCKETS: usize = 8;
    const BUCKET_DIV: usize = 32_usize.div_ceil(Self::N_BUCKETS);
    const LANES: usize = i16x32::LANES as usize;
    const NNUE2SCORE: i32 = 400;
    const INPUT_SCALE: i32 = 255;
    const HIDDEN_SCALE: i32 = 64;
    const COMB_SCALE: i32 = Self::HIDDEN_SCALE * Self::INPUT_SCALE;

    const W_I16: usize = Self::N_KING_BUCKETS * Self::N_INPUTS * Self::L1
        + Self::L1
        + Self::N_BUCKETS * (2 * Self::L1);
    const B_I16: usize = Self::N_BUCKETS;
    const WDL_F32: usize = Self::N_BUCKETS * (3 * (2 * Self::L1) + 3);
    const NET_BYTES: usize = 2 * Self::W_I16 + 2 * Self::B_I16 + 4 * Self::WDL_F32;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_idx(pc: Piece, sq: SQ, color: Color, perspective: Perspective) -> usize {
        perspective.bucket * Network::N_INPUTS + perspective.feature_idx(pc, sq, color)
    }

    // Position "6k1/8/8/8/8/8/8/1K6 w - - 0 1": white king b1 (files a-d,
    // not mirrored), black king g8 (mirrored to b8 -> relative b1); both
    // perspectives in bucket 0.
    #[test]
    fn feature_indices_match_trainer_convention() {
        let wk = Piece::make_piece(Color::White, PieceType::King);
        let bk = Piece::make_piece(Color::Black, PieceType::King);

        let w_perspective = Perspective::new(SQ::B1);
        let b_perspective = Perspective::new(SQ::G8.relative(Color::Black));

        // White perspective.
        assert_eq!(flat_idx(wk, SQ::B1, Color::White, w_perspective), 321);
        assert_eq!(flat_idx(bk, SQ::G8, Color::White, w_perspective), 766);

        // Black perspective: piece colors flip, ranks flip, and the black
        // king's file mirrors the whole board.
        assert_eq!(flat_idx(wk, SQ::B1, Color::Black, b_perspective), 766);
        assert_eq!(flat_idx(bk, SQ::G8, Color::Black, b_perspective), 321);
    }

    // Position "6k1/8/8/3K4/8/8/8/8 w - - 0 1": white king d5 (rank 5,
    // bucket 3), black king g8 (mirrored to b1, bucket 0). Exercises the
    // up-board buckets for the white perspective only.
    #[test]
    fn feature_indices_bucket_offsets_match_trainer_convention() {
        let wk = Piece::make_piece(Color::White, PieceType::King);
        let bk = Piece::make_piece(Color::Black, PieceType::King);

        let w_perspective = Perspective::new(SQ::D5);
        let b_perspective = Perspective::new(SQ::G8.relative(Color::Black));

        assert_eq!(w_perspective.bucket, 3);
        assert_eq!(b_perspective.bucket, 0);

        // White perspective: every feature comes from bucket 3.
        assert_eq!(flat_idx(bk, SQ::G8, Color::White, w_perspective), 3070);
        assert_eq!(flat_idx(wk, SQ::D5, Color::White, w_perspective), 2659);

        // Black perspective: bucket 0, mirrored.
        assert_eq!(flat_idx(bk, SQ::G8, Color::Black, b_perspective), 321);
        assert_eq!(flat_idx(wk, SQ::D5, Color::Black, b_perspective), 732);
    }

    #[test]
    fn clone_shares_weights_with_the_static_blob() {
        let a = Network::new();
        let b = a.clone();

        // Same pointers after clone, and they point into NETWORK itself.
        assert!(std::ptr::eq(
            a.embeddings[0].weights,
            b.embeddings[0].weights
        ));
        assert!(std::ptr::eq(
            a.hidden_layers[0].weights,
            b.hidden_layers[0].weights
        ));
        assert!(std::ptr::eq(
            a.wdl_layers[0].weights,
            b.wdl_layers[0].weights
        ));

        let blob = NETWORK.0.as_ptr_range();
        let w = a.embeddings[0].weights.as_ptr() as *const u8;
        assert!(blob.contains(&w));

        println!(
            "size_of::<Network>() = {} bytes (blob = {} bytes)",
            std::mem::size_of::<Network>(),
            Network::NET_BYTES,
        );
    }
}
