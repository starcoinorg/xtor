use pin_project_lite::pin_project;
use std::pin::Pin;
use tracing::Span;
use xtor::Actor;

pub trait ActorInstrument: Sized {
    /// Instruments this type with the provided `Span`, returning an
    /// `ActorInstrumented` wrapper.
    ///
    /// When the wrapped actor future is polled, the attached `Span`
    /// will be entered for the duration of the poll.
    fn actor_instrument(self, span: Span) -> ActorInstrumented<Self> {
        ActorInstrumented { inner: self, span }
    }

    #[inline]
    fn in_current_actor_span(self) -> ActorInstrumented<Self> {
        self.actor_instrument(Span::current())
    }
}

impl<T: Sized> ActorInstrument for T {}

pin_project! {
    /// An actor future that has been instrumented with a `tracing` span.
    #[derive(Debug, Clone)]
    pub struct ActorInstrumented<T>
    {
        #[pin]
        inner: T,
        span: Span,
    }
}

impl<T: Actor> Actor for ActorInstrumented<T> {}

impl<T> ActorInstrumented<T> {
    /// Borrows the `Span` that this type is instrumented by.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Mutably borrows the `Span` that this type is instrumented by.
    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    /// Borrows the wrapped type.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Mutably borrows the wrapped type.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Get a pinned reference to the wrapped type.
    pub fn inner_pin_ref(self: Pin<&Self>) -> Pin<&T> {
        self.project_ref().inner
    }

    /// Get a pinned mutable reference to the wrapped type.
    pub fn inner_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.project().inner
    }

    /// Consumes the `Instrumented`, returning the wrapped type.
    ///
    /// Note that this drops the span.
    pub fn into_inner(self) -> T {
        self.inner
    }
}
