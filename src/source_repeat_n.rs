use rodio::source::{Buffered, SeekError};
use rodio::{Sample, Source};
use std::time::Duration;

pub struct RepeatN<I, F>
where
    I: Source,
    I::Item: Sample,
    F: FnOnce() -> (),
{
    times_left: usize,
    inner: Buffered<I>,
    next: Buffered<I>,
    on_end: Option<F>,
}

impl<I, F> RepeatN<I, F>
where
    I: Source,
    I::Item: Sample,
    F: FnOnce() -> (),
{
    pub fn new(i: I, times: usize, on_end: F) -> Self {
        let i = i.buffered();

        RepeatN {
            inner: i.clone(),
            next: i,
            times_left: times,
            on_end: Some(on_end),
        }
    }
}

impl<I, F> Iterator for RepeatN<I, F>
where
    I: Source,
    I::Item: Sample,
    F: FnOnce() -> (),
{
    type Item = <I as Iterator>::Item;

    #[inline]
    fn next(&mut self) -> Option<<I as Iterator>::Item> {
        if let Some(value) = self.inner.next() {
            return Some(value);
        }

        if self.times_left == 0 {
            return None;
        }

        self.times_left -= 1;

        if self.times_left == 0 {
            self.on_end.take().map(|f| f.call_once(()));
        }

        self.inner = self.next.clone();
        self.inner.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.times_left, Some(self.times_left))
    }
}

impl<I, F> Source for RepeatN<I, F>
where
    I: Iterator + Source,
    I::Item: Sample,
    F: FnOnce() -> (),
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        match self.inner.current_frame_len() {
            Some(0) => self.next.current_frame_len(),
            a => a,
        }
    }

    #[inline]
    fn channels(&self) -> u16 {
        match self.inner.current_frame_len() {
            Some(0) => self.next.channels(),
            _ => self.inner.channels(),
        }
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        match self.inner.current_frame_len() {
            Some(0) => self.next.sample_rate(),
            _ => self.inner.sample_rate(),
        }
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }

    #[inline]
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.inner.try_seek(pos)
    }
}

impl<I, F> Clone for RepeatN<I, F>
where
    I: Source,
    I::Item: Sample,
    F: FnOnce() -> () + Clone,
{
    #[inline]
    fn clone(&self) -> RepeatN<I, F> {
        RepeatN {
            times_left: self.times_left,
            inner: self.inner.clone(),
            next: self.next.clone(),
            on_end: self.on_end.clone(),
        }
    }
}
