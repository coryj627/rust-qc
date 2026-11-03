use std::cell::Cell; use std::marker::PhantomData;
struct IsSync<'a,T>{is_sync:&'a Cell<bool>,_marker:PhantomData<T>}
impl<T> Clone for IsSync<'_,T>{fn clone(&self)->Self{self.is_sync.set(false);IsSync{is_sync:self.is_sync,_marker:PhantomData}}}
impl<T:Sync> Copy for IsSync<'_,T>{}
fn is_sync<T>()->bool{let is_sync=Cell::new(true);let _=[IsSync::<T>{is_sync:&is_sync,_marker:PhantomData}].clone();is_sync.get()}
fn main(){println!("String {} u32 {} Cell {}",is_sync::<String>(),is_sync::<u32>(),is_sync::<Cell<u32>>());}
