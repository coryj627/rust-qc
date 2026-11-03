#![allow(unused)]
use mlua::prelude::*; use mlua::{UserDataRef,UserDataRefMut,Function,AnyUserData,UserData};
use std::cell::Cell; use std::rc::Rc; use std::sync::{Arc,atomic::*}; use std::thread;
type R=LuaResult<()>;
fn req<T:Sync>(){}
fn sync_cell()->R{req::<UserDataRef<Cell<u64>>>();req::<UserDataRefMut<Cell<u64>>>();let lua=Lua::new();let ud=lua.create_any_userdata(Cell::new(0u64))?;let r=ud.borrow::<Cell<u64>>()?;
  thread::scope(|s|{for _ in 0..4{s.spawn(||for _ in 0..5_000_000{r.set(std::hint::black_box(r.get())+1);});}});println!("expected 20000000 got {}",r.get());Ok(())}
fn sync_cell_mut()->R{let lua=Lua::new();let ud=lua.create_any_userdata(Cell::new(0u64))?;let r=ud.borrow_mut::<Cell<u64>>()?;
  thread::scope(|s|{for _ in 0..4{s.spawn(||for _ in 0..5_000_000{r.set(std::hint::black_box(r.get())+1);});}});println!("expected 20000000 got {}",r.get());Ok(())}
fn scope_race()->R{let lua=Lua::new();let main=thread::current().id();let hits=Arc::new(AtomicUsize::new(0));let stop=Arc::new(AtomicBool::new(false));
  let slot:Arc<parking::Slot>=Arc::new(parking::Slot::default());
  let (s2,st2)=(slot.clone(),stop.clone());
  let h=thread::spawn(move||{let mut calls=0u64;while !st2.load(Ordering::Relaxed){let f=s2.0.lock().unwrap().clone();if let Some(f)=f{let _=f.call::<()>(());calls+=1;}}calls});
  for i in 0..200000u32{let hc=hits.clone();let rc=Rc::new(0);lua.scope(|sc|{let rc=rc.clone();let f=sc.create_function(move|_,()|{let _c=rc.clone();if thread::current().id()!=main{hc.fetch_add(1,Ordering::SeqCst);}Ok(())})?;*slot.0.lock().unwrap()=Some(f);Ok(())})?;*slot.0.lock().unwrap()=None;}
  stop.store(true,Ordering::Relaxed);let calls=h.join().unwrap();println!("worker calls {} ran scoped !Send closure on worker {}",calls,hits.load(Ordering::SeqCst));Ok(())}
mod parking{#[derive(Default)]pub struct Slot(pub std::sync::Mutex<Option<mlua::Function>>);}
fn take_race()->R{let lua=Lua::new();for i in 0..200000{let ud=lua.create_any_userdata(1u64)?;let r=ud.borrow::<u64>()?;let t=thread::spawn(move||drop(r));loop{match ud.take::<u64>(){Ok(_)=>break,Err(_)=>{std::hint::spin_loop();}}}t.join().unwrap();}println!("no panic");Ok(())}
fn two_borrow()->R{let lua=Lua::new();let ud=lua.create_any_userdata(1u64)?;let a=ud.borrow::<u64>()?;println!("second {:?}",ud.borrow::<u64>().map(|_|()));Ok(())}
fn main(){let a=std::env::args().nth(1).unwrap();let r=match a.as_str(){"sync_cell"=>sync_cell(),"sync_cell_mut"=>sync_cell_mut(),"scope_race"=>scope_race(),"take_race"=>take_race(),"two_borrow"=>two_borrow(),_=>panic!()};println!("result: {:?}",r.map_err(|e|e.to_string()));}
