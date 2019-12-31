use arraydeque::{ArrayDeque, Saturating, Array};

pub struct Stream<T, L, A>
    where
        T: Clone,
        L: Iterator<Item=T>,
        A: Array<Item=Option<T>>
{
    input: L,
    current: Option<T>,
    buffer: ArrayDeque<A, Saturating>,
}

impl<T, L, A> Stream<T, L, A>
    where
        T: Clone,
        L: Iterator<Item=T>,
        A: Array<Item=Option<T>>
{
    pub fn new(input: L) -> Self {
        Self {
            input,
            current: None,
            buffer: ArrayDeque::new(),
        }
    }

    pub fn consume(&mut self) -> Option<T> {
        self.current = match self.buffer.pop_front() {
            Some(value) => value,
            None => self.input.next(),
        };
        self.current.clone()
    }

    pub fn reconsume_current(&mut self) {
        self.buffer.push_front(self.current.clone()).unwrap()
    }
}

pub trait PeekAt<T> {
    fn peek_at(&mut self, index: i32) -> T;
}

impl<T, L, A> PeekAt<Option<T>> for Stream<T, L, A>
    where T: Clone, L: Iterator<Item=T>, A: Array<Item=Option<T>>
{
    fn peek_at(&mut self, index: i32) -> Option<T> {
        if index < -1 {
            panic!();
        } else if index == -1 {
            self.current.clone()
        } else {
            // Fill buffer until it contains index
            while (index + 1) as usize > self.buffer.len() {
                self.buffer.push_back(self.input.next());
            }
            (*self.buffer.get(index as usize).unwrap()).clone()
        }
    }
}

pub trait PeekTuple<R> {
    fn peek_tuple(&mut self, index: i32) -> R;
}

impl<T, U> PeekTuple<(T, T)> for U where T: Clone, U: PeekAt<T> {
    fn peek_tuple(&mut self, index: i32) -> (T, T) {
        (self.peek_at(index), self.peek_at(index + 1))
    }
}

impl<T, U> PeekTuple<(T, T, T)> for U where T: Clone, U: PeekAt<T> {
    fn peek_tuple(&mut self, index: i32) -> (T, T, T) {
        (self.peek_at(index), self.peek_at(index + 1), self.peek_at(index + 2))
    }
}