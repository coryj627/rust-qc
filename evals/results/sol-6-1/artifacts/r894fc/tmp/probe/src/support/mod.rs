use std::{future::Future, sync::Arc, task::{Wake,Waker,Context,Poll}};
pub fn block_on<F:Future>(f:F)->F::Output {
    struct W(std::thread::Thread);
    impl Wake for W { fn wake(self:Arc<Self>) {self.0.unpark();} }
    let w=Waker::from(Arc::new(W(std::thread::current())));let mut cx=Context::from_waker(&w);let mut f=Box::pin(f);
    loop {match f.as_mut().poll(&mut cx) {Poll::Ready(v)=>return v,Poll::Pending=>std::thread::park()}}
}
