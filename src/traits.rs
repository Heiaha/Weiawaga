use super::piece::*;

pub trait Mirror: Copy + Sized {
    fn mirror(&self) -> Self;

    fn relative(&self, color: Color) -> Self {
        match color {
            Color::White => *self,
            Color::Black => self.mirror(),
        }
    }
}
