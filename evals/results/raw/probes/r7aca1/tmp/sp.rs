use std::cell::Cell; use std::marker::PhantomData;
struct IsSync<'a,T>{is_sync:&'a Cell<bool>,_m:PhantomData<T>}
impl<T> Clone for IsSync<'_,T>{fn clone(&self)->Self{self.is_sync.set(false);IsSync{is_sync:self.is_sync,_m:PhantomData}}}
impl<T:Sync> Copy for IsSync<'_,T>{}
fn is_sync<T>()->bool{let c=Cell::new(true);let _=[IsSync::<T>{is_sync:&c,_m:PhantomData}].clone();c.get()}
fn main(){println!("u64 {} Cell {}",is_sync::<u64>(),is_sync::<Cell<u64>>());}
