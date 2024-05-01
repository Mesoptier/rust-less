use std::fmt::Debug;
use std::iter::Enumerate;
use std::num::NonZeroUsize;
use std::slice::Iter;
use winnow::error::Needed;
use winnow::stream::{Offset, Stream, StreamIsPartial};

#[derive(Debug, PartialEq)]
pub(crate) struct RefStream<'t, T> {
    tokens: &'t [T],
}

impl<'t, T> RefStream<'t, T> {
    pub fn new(tokens: &'t [T]) -> Self {
        Self { tokens }
    }

    pub fn into_inner(self) -> &'t [T] {
        self.tokens
    }
}

impl<'t, T> Clone for RefStream<'t, T> {
    fn clone(&self) -> Self {
        Self {
            tokens: self.tokens,
        }
    }
}

impl<'t, T> Offset<Self> for RefStream<'t, T> {
    fn offset_from(&self, start: &Self) -> usize {
        let first = start.tokens.as_ptr();
        let second = self.tokens.as_ptr();
        debug_assert!(first <= second);
        (second as usize - first as usize) / std::mem::size_of::<T>()
    }
}

impl<'t, T: Debug> Stream for RefStream<'t, T> {
    type Token = &'t T;
    type Slice = &'t [T];
    type IterOffsets = Enumerate<Iter<'t, T>>;
    type Checkpoint = Self;

    fn iter_offsets(&self) -> Self::IterOffsets {
        self.tokens.iter().enumerate()
    }

    fn eof_offset(&self) -> usize {
        self.tokens.len()
    }

    fn next_token(&mut self) -> Option<Self::Token> {
        let (token, next) = self.tokens.split_first()?;
        self.tokens = next;
        Some(token)
    }

    fn offset_for<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Token) -> bool,
    {
        self.tokens.iter().position(predicate)
    }

    fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
        if let Some(needed) = tokens
            .checked_sub(self.tokens.len())
            .and_then(NonZeroUsize::new)
        {
            Err(Needed::Size(needed))
        } else {
            Ok(tokens)
        }
    }

    fn next_slice(&mut self, offset: usize) -> Self::Slice {
        let (slice, next) = self.tokens.split_at(offset);
        self.tokens = next;
        slice
    }

    fn checkpoint(&self) -> Self::Checkpoint {
        self.clone()
    }

    fn reset(&mut self, checkpoint: &Self::Checkpoint) {
        *self = checkpoint.clone();
    }

    fn raw(&self) -> &dyn Debug {
        self
    }
}

impl<'t, T> StreamIsPartial for RefStream<'t, T> {
    type PartialState = ();

    fn complete(&mut self) -> Self::PartialState {}

    fn restore_partial(&mut self, _state: Self::PartialState) {}

    fn is_partial_supported() -> bool {
        false
    }
}
