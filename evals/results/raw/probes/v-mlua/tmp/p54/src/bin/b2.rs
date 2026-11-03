#![allow(unused)]
use mlua::prelude::*;
use mlua::{Value,Nil,UserData,UserDataMethods,Function,Table,MultiValue,AnyUserData,VmState,HookTriggers,LuaOptions,StdLib};
use std::future::Future; use std::task::{Context,Waker,Poll}; use std::rc::Rc; use std::sync::{Arc,atomic::*};
type R=LuaResult<()>;
struct Y(bool); impl Future for Y{type Output=();fn poll(mut self:std::pin::Pin<&mut Self>,cx:&mut Context<'_>)->Poll<()>{if self.0{Poll::Ready(())}else{self.0=true;cx.waker().wake_by_ref();Poll::Pending}}}
fn destroy2()->R{struct D(String);impl UserData for D{fn add_methods<M:UserDataMethods<Self>>(m:&mut M){m.add_method("g",|_,this,ud:AnyUserData|{let r=ud.borrow::<D>()?;let e=ud.destroy();println!("destroy {:?}",e);drop(r);
   let v:Vec<Vec<u8>>=(0..200).map(|_|vec![0x5A;64]).collect();println!("after: len={} first16={:?}",this.0.len(),String::from_utf8_lossy(&this.0.as_bytes()[..16]));std::mem::forget(v);Ok(())});}}
  let lua=Lua::new();let u=lua.create_userdata(D("hello-hello-hello-hello-hello-hello-hello-hello-hello-hello-1234".into()))?;lua.globals().set("v",u)?;println!("{:?}",lua.load("v:g(v)").exec().map_err(|e|e.to_string()));Ok(())}
fn co_global2()->R{let lua=Lua::new();lua.load("local y=coroutine.yield; coroutine.yield=function(...) hijacked=true return y(...) end").exec()?;
  let f=lua.create_async_function(|_,()|async{Y(false).await;Ok(1)})?;lua.globals().set("f",f)?;let g:Function=lua.load("return f()").into_function()?;
  let w=Waker::noop();let mut cx=Context::from_waker(&w);let mut fut=Box::pin(g.call_async::<i32>(()));println!("{:?}",fut.as_mut().poll(&mut cx).is_ready());println!("hijacked={:?}",lua.globals().get::<Value>("hijacked"));Ok(())}
fn recycle2()->R{let lua=Lua::new_with(StdLib::ALL_SAFE,LuaOptions::new().thread_pool_size(4))?;
  let f:Function=lua.load("for i=1,100 do end return 1").into_function()?;let w=Waker::noop();let mut cx=Context::from_waker(&w);
  {let mut fut=Box::pin(f.call_async::<i32>(()));println!("pre {}",fut.as_mut().poll(&mut cx).is_ready());}
  let c=Arc::new(AtomicUsize::new(0));let c2=c.clone();lua.set_global_hook(HookTriggers::new().every_nth_instruction(1),move|_,_|{c2.fetch_add(1,Ordering::SeqCst);Ok(VmState::Continue)})?;
  {let mut fut=Box::pin(f.call_async::<i32>(()));fut.as_mut().poll(&mut cx);println!("recycled thread: hook fired {}",c.swap(0,Ordering::SeqCst));std::mem::forget(fut);}
  {let mut fut=Box::pin(f.call_async::<i32>(()));fut.as_mut().poll(&mut cx);println!("new thread: hook fired {}",c.swap(0,Ordering::SeqCst));std::mem::forget(fut);}
  let t=lua.create_thread(f.clone())?;t.resume::<i32>(())?;println!("create_thread: hook fired {}",c.swap(0,Ordering::SeqCst));Ok(())}
#[derive(Clone)]struct TA(String);impl UserData for TA{}
struct O<const N:usize>(u64,u64,u64,u64);impl<const N:usize> UserData for O<N>{}
unsafe fn mtp(lua:&Lua,ud:&AnyUserData)->usize{let v:mlua::Integer=lua.exec_raw(ud.clone(),|st|{mlua::ffi::lua_getmetatable(st,-1);let p=mlua::ffi::lua_topointer(st,-1) as usize;mlua::ffi::lua_pop(st,2);mlua::ffi::lua_pushinteger(st,p as _);}).unwrap();v as usize}
fn stale2()->R{
 for k in 0..16usize{ for n in 0..24usize{
   let lua=Lua::new();lua.register_userdata_type::<TA>(|_|{})?;let ua=lua.create_any_userdata(TA("a".into()))?;drop(ua.borrow::<TA>()?);let old=unsafe{mtp(&lua,&ua)};
   lua.register_userdata_type::<TA>(|_|{})?;
   lua.register_userdata_type::<O<0>>(|_|{})?;
   let g:Vec<_>=(0..k).map(|_|lua.create_table().unwrap()).collect();drop(g);
   drop(ua);lua.gc_collect()?;lua.gc_collect()?;
   let mut keep=vec![];for _ in 0..n{keep.push(lua.create_table()?);}
   let ub=lua.create_any_userdata(O::<0>(0x4242424242424242,0x4242424242424242,0x4242424242424242,0x4242424242424242))?;
   if unsafe{mtp(&lua,&ub)}==old{println!("garbage={k} filler={n}: same mt address. O.is::<TA>={} O.is::<O>={}",ub.is::<TA>(),ub.is::<O<0>>());
     match ub.borrow::<TA>(){Ok(r)=>{println!("borrow::<TA> Ok: String len={:x}",r.0.len());},Err(e)=>println!("{e}")};return Ok(())}
 }}
 println!("no mt address reuse");Ok(())}
fn stale_safe()->R{
 // same flow, only safe API, no address read
 for k in 0..16usize{ for n in 0..24usize{
   let lua=Lua::new();lua.register_userdata_type::<TA>(|_|{})?;let ua=lua.create_any_userdata(TA("a".into()))?;drop(ua.borrow::<TA>()?);
   lua.register_userdata_type::<TA>(|_|{})?;lua.register_userdata_type::<O<0>>(|_|{})?;
   let g:Vec<_>=(0..k).map(|_|lua.create_table().unwrap()).collect();drop(g);
   drop(ua);lua.gc_collect()?;lua.gc_collect()?;
   let mut keep=vec![];for _ in 0..n{keep.push(lua.create_table()?);}
   let ub=lua.create_any_userdata(O::<0>(0x4242424242424242,0x4242424242424242,0x4242424242424242,0x4242424242424242))?;
   if ub.is::<TA>(){println!("safe API only, garbage={k} filler={n}: O.is::<TA>=true O.is::<O>={}",ub.is::<O<0>>());
     match ub.borrow::<TA>(){Ok(r)=>{println!("borrow::<TA> Ok: String len={:x}",r.0.len());let _=r.0.as_bytes().first();println!("reading string bytes");println!("{}",r.0.bytes().count());},Err(e)=>println!("{e}")};return Ok(())}
 }}
 println!("no confusion");Ok(())}
fn leak2()->R{let lua=Lua::new();let rc=Rc::new(1);lua.set_memory_limit(1)?;let r=lua.create_function({let rc=rc.clone();move|_,()|{let _=&rc;Ok(())}});println!("fn {:?} strong now {}",r.as_ref().map(|_|()).map_err(|e|e.to_string()),Rc::strong_count(&rc));drop(r);lua.set_memory_limit(0)?;lua.gc_collect()?;lua.gc_collect()?;println!("strong={}",Rc::strong_count(&rc));Ok(())}
fn gcgen()->R{let lua=Lua::new();let f=lua.create_function(|lua,()|{lua.gc_gen(0,0);Ok(())})?;lua.globals().set("f",f)?;println!("{:?}",lua.load("setmetatable({}, {__gc=function() print('gc',pcall(f)) end}); collectgarbage(); collectgarbage()").exec().map_err(|e|e.to_string()));Ok(())}
fn hugealloc()->R{let lua=Lua::new();println!("{:?}",lua.load("return pcall(string.rep,'x',1<<45)").eval::<MultiValue>().map(|m|format!("{:?}",m)).map_err(|e|e.to_string()));Ok(())}
fn hugetab()->R{let lua=Lua::new();println!("{:?}",lua.create_table_with_capacity(usize::MAX,0).map(|_|()).map_err(|e|e.to_string()));Ok(())}
fn seqstack()->R{let lua=Lua::new();let f:Function=lua.load("local t={} for i=2,40 do t[i]=i end t[1]={1,2,3} return table.unpack(t)").into_function()?;
 let cb=lua.create_function(|lua,mv:MultiValue|{let n=mv.len();let v:Vec<Value>=mv.into_iter().collect();Ok(n)})?;println!("{:?}",f.call::<(Table,mlua::Variadic<i32>)>(()).map(|x|x.1.len()));Ok(())}
fn main(){let a=std::env::args().nth(1).unwrap();let r=match a.as_str(){"destroy2"=>destroy2(),"co_global2"=>co_global2(),"recycle2"=>recycle2(),"stale2"=>stale2(),"hugealloc"=>hugealloc(),"hugetab"=>hugetab(),"seqstack"=>seqstack(),"stale_safe"=>stale_safe(),"leak2"=>leak2(),"gcgen"=>gcgen(),_=>panic!()};println!("result: {:?}",r.map_err(|e|e.to_string()));}
