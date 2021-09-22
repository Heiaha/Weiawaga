pub enum Color {
    WHITE = 0,
    BLACK = 1,
}

impl Not for Color {
    type Output = Color;

    #[inline]
    fn not(self) -> Color {
        Color(BLACK ^ side)
    }
}