#![allow(unused)]
use mlua::prelude::*;
use mlua::{Value, Nil, UserData, UserDataMethods, Function, Table, Variadic, MultiValue, AnyUserData, VmState, HookTriggers, ThreadStatus, DeserializeOptions, SerializeOptions, LuaOptions, StdLib, IntoLua, ObjectLike};
use std::collections::*;
use std::rc::Rc;
use std::path::{Path,PathBuf};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context,Waker,Poll};
type R = LuaResult<()>;

struct S(String);
impl UserData for S { fn add_methods<M: UserDataMethods<Self>>(m:&mut M){
    m.add_method("get",|_,this,()|{println!("method sees len={} cap={}",this.0.len(),this.0.capacity()); Ok(this.0.clone())});
}}
fn scope_uaf()->R{ let lua=Lua::new();
  lua.scope(|sc|{ let ud=sc.create_userdata(S("hello world hello world hello world".into()))?; lua.globals().set("ud",ud)?; lua.load("m=ud.get; print(m(ud))").exec()})?;
  println!("after scope: ud:get -> {:?}", lua.load("return pcall(function() return ud:get() end)").eval::<MultiValue>());
  lua.load("for i=1,50 do local _=string.rep('Z',32+i) end").exec()?;
  println!("saved: {:?}", lua.load("return m(ud)").eval::<String>()); Ok(())}
fn scope_uaf_any()->R{ let lua=Lua::new();
  
  lua.scope(|sc|{ let ud=sc.create_any_userdata(vec![1u8;32],|r|{r.add_method_mut("push",|_,t,()|{println!("len={}",t.len());t.push(1);Ok(t.len())});})?; lua.globals().set("ud",ud)?; lua.load("m=ud.push; print(m(ud))").exec()})?;
  println!("saved: {:?}", lua.load("return m(ud)").eval::<usize>()); Ok(())}
fn foreign_mt()->R{ let a=Lua::new(); let b=Lua::new();
  let t=a.create_table()?; let s=a.create_string("x")?;
  let _p=b.create_table()?; let tb=b.create_table()?;
  t.set_metatable(Some(tb))?; println!("no panic; mt={:?}",t.metatable()); Ok(())}
fn foreign_thread()->R{ let a=Lua::new(); let b=Lua::new();
  let fa=a.load("return 'function from lua A'").into_function()?;
  let fb=b.load("return 'function from lua B'").into_function()?;
  let th=a.create_thread(fb)?; println!("{:?}",th.resume::<String>(())); drop(fa); Ok(())}
#[derive(Clone)] struct TA(String); impl UserData for TA{}
#[derive(Clone)] struct TB(u64,u64,u64,u64); impl UserData for TB{}
fn stale()->R{ for n in 0..200 { let lua=Lua::new();
   lua.register_userdata_type::<TA>(|_|{})?;
   let ua=lua.create_any_userdata(TA("a".into()))?; let _=ua.borrow::<TA>()?; 
   lua.register_userdata_type::<TA>(|_|{})?;
   drop(ua); lua.gc_collect()?; lua.gc_collect()?;
   let mut keep=vec![]; for _ in 0..n { keep.push(lua.create_table()?); }
   lua.register_userdata_type::<TB>(|_|{})?;
   let ub=lua.create_any_userdata(TB(0x4242424242424242,1,2,3))?;
   if ub.is::<TA>() { println!("n={} confusion: is::<TA>={} is::<TB>={}",n,ub.is::<TA>(),ub.is::<TB>());
     if let Ok(r)=ub.borrow::<TA>(){ println!("borrow as TA ok; len={:x}",r.0.len()); } return Ok(()); } }
  println!("no confusion"); Ok(())}
fn nil_set()->R{let lua=Lua::new();let t=lua.create_table()?; println!("{:?}",t.set(Nil,1)); Ok(())}
fn nan_rawset()->R{let lua=Lua::new();let t=lua.create_table()?; println!("{:?}",t.raw_set(f64::NAN,1)); Ok(())}
fn nil_rawremove()->R{let lua=Lua::new();let t=lua.create_table()?; println!("{:?}",t.raw_remove(Nil)); Ok(())}
fn nil_from()->R{let lua=Lua::new(); println!("{:?}",lua.create_table_from(vec![(Nil,1)]).map(|_|())); Ok(())}
fn nil_tovalue()->R{let lua=Lua::new(); let mut m=BTreeMap::new(); m.insert(None::<i32>,1);
  println!("{:?}",lua.to_value_with(&m,SerializeOptions::new().serialize_none_to_null(false)).map(|_|())); Ok(())}
fn nil_limit()->R{let lua=Lua::new(); lua.set_memory_limit(1<<30)?;let t=lua.create_table()?; println!("{:?}",t.set(Nil,1)); Ok(())}
fn nil_cb()->R{let lua=Lua::new(); struct G; impl Drop for G{fn drop(&mut self){println!("guard dropped")}}
  let f=lua.create_function(|lua,(t,k):(Table,Value)|{let _g=G; t.set(k,1)?; println!("after set"); Ok(())})?; lua.globals().set("f",f)?;
  println!("{:?}",lua.load("return pcall(f,{},nil)").eval::<MultiValue>());
  let cur=lua.current_thread();println!("main status {:?}",cur.status());
  let f2=lua.load("local co=coroutine.create(function() return pcall(f,{},nil) end); return coroutine.resume(co)").eval::<MultiValue>();println!("{:?}",f2);
  println!("current==main {}",lua.current_thread()==cur); Ok(())}
fn mut_pairs()->R{let lua=Lua::new();let t=lua.create_table()?;for i in 0..8{t.set(format!("k{i}"),i)?;}
  for p in t.clone().pairs::<String,i32>(){ let(k,_)=p?; t.set(k,Nil)?; for j in 0..200{t.set(format!("n{j}"),j)?;} lua.gc_collect()?; } println!("done"); Ok(())}
fn mut_foreach()->R{let lua=Lua::new();let t=lua.create_table()?;for i in 0..8{t.set(format!("k{i}"),i)?;}
  let t2=t.clone(); t.for_each(|k:String,_:i32|{ t2.set(k,Nil)?; for j in 0..200{t2.set(format!("n{j}"),j)?;} lua.gc_collect()?; Ok(())})?; println!("done"); Ok(())}
fn err_key(mode:u8)->R{let lua=Lua::new(); let bad=lua.create_function(|_,()|->LuaResult<()>{Err(LuaError::runtime("x"))})?; lua.globals().set("bad",bad)?;
  let t:Table=lua.load("local ok,e=pcall(bad); return {[e]=1,x=2}").eval()?;
  match mode {0=>{for p in t.pairs::<Value,Value>(){println!("pair {:?}",p.map(|_|()));}},
   1=>{println!("{:#?}",t);},
   _=>{let r:LuaResult<serde_json::Value>=lua.from_value_with(Value::Table(t),DeserializeOptions::new().deny_unsupported_types(false));println!("{:?}",r);}}
  println!("survived"); Ok(())}
fn deep(mode:u8,d:u32)->R{let lua=Lua::new(); let v:Value=lua.load(format!("local t={{}} for i=1,{d} do t={{t}} end return t")).eval()?;
  match mode{0=>{println!("{}",serde_json::to_string(&v).map(|s|s.len()).unwrap_or(0));},1=>{let j:serde_json::Value=lua.from_value(v)?;println!("ok");std::mem::forget(j);},_=>{println!("{}",format!("{:#?}",v).len());}} Ok(())}
struct PD; impl Drop for PD{fn drop(&mut self){panic!("drop panic")}} impl UserData for PD{}
fn drop_panic()->R{let lua=Lua::new(); {let ud=lua.create_userdata(PD)?; drop(ud);}
  let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||lua.gc_collect())); println!("caught {}",r.is_err());
  println!("{:?}",lua.load("return collectgarbage('count')").eval::<Value>());
  let before=lua.used_memory(); for _ in 0..20000{lua.create_table()?;} lua.gc_collect()?; println!("before {} after {}",before,lua.used_memory());
  println!("{:?}",lua.load("return debug and 1 or 2").eval::<Value>());
  println!("{:?}",lua.load("error('tb')").exec().map_err(|e|e.to_string()));
  let t=lua.create_table()?; println!("set nil next (unprotected)"); let _=t.set(Nil,1); println!("no crash"); Ok(())}
fn warn_err()->R{let lua=Lua::new(); lua.set_warning_function(|_,_,_|Err(LuaError::runtime("w"))); lua.warning("hello",false); println!("survived"); Ok(())}
fn warn_panic()->R{let lua=Lua::new(); lua.set_warning_function(|_,_,_|panic!("w")); lua.warning("hello",false); println!("survived"); Ok(())}
fn warn_co()->R{let lua=Lua::new(); lua.set_warning_function(|_,_,_|Err(LuaError::runtime("w")));
  println!("{:?}",lua.load("local co=coroutine.create(function() warn('x') end); local ok,e=coroutine.resume(co); return 'resume returned',ok,tostring(e)").eval::<MultiValue>().map_err(|e|e.to_string())); Ok(())}
fn normal_status(which:u8)->R{let lua=Lua::new();
  let a:mlua::Thread=lua.load("return coroutine.create(function() inner() return 'A done' end)").eval()?; 
  let a2=a.clone(); let fnreset=lua.create_function(|_,()|Ok(()))?;
  let inner=lua.create_function(move|lua,()|{ 
     let b=lua.create_thread(lua.create_function({let a=a2.clone();let fr=fnreset.clone();move|lua,()|{
       println!("A.status from B={:?}",a.status());
       let n:String=lua.load("return coroutine.status(acoro)").eval()?; println!("lua says {}",n);
       if which==1 {println!("resume: {:?}",a.resume::<()>(()));}
       if which==2 {println!("reset: {:?}",a.reset(fr.clone()));}
       Ok(())}})?)?;
     b.resume::<()>(())?; Ok(())})?;
  lua.globals().set("inner",inner)?; lua.globals().set("acoro",a.clone())?;
  println!("{:?}",a.resume::<String>(())); Ok(())}
fn noop_waker()->Waker{Waker::noop().clone()}
fn call_async_drop(hang:bool)->R{let lua=Lua::new(); let f:Function=lua.load(if hang {"coroutine.yield(); while true do end"} else {"local v=coroutine.yield(); ran_after_drop=tostring(v)"}).into_function()?;
  {let mut fut=Box::pin(f.call_async::<()>(())); let w=noop_waker(); let mut cx=Context::from_waker(&w); let r=fut.as_mut().poll(&mut cx); println!("poll {:?}",r.is_pending()); drop(fut);}
  println!("ran_after_drop={:?}",lua.globals().get::<Value>("ran_after_drop")); Ok(())}
fn eqslice()->R{let lua=Lua::new();let t:Table=lua.load("return {1,2}").eval()?;let e=lua.create_table()?;println!("{} {}",t==[1,2,3],e==[1,2,3]); println!("{}",t==[1]); Ok(())}
fn sparse()->R{let lua=Lua::new(); for s in ["{[5]=1}","{[1000]=1}","{[5000000]=1}"]{let v:Value=lua.load(format!("return {s}")).eval()?; let j=serde_json::to_string(&v.to_serializable().detect_mixed_tables(true));println!("{s} {:?}",j.map(|x|x.len()));}
  let v:Value=lua.load("return {}").eval()?; println!("{}",serde_json::to_string(&v.to_serializable().detect_mixed_tables(true).encode_empty_tables_as_array(false)).unwrap());
  println!("no-detect:{}",serde_json::to_string(&v.to_serializable().encode_empty_tables_as_array(false)).unwrap());
  lua.globals().set("am",lua.array_metatable())?; let v:Value=lua.load("return setmetatable({a=5},am)").eval()?; println!("{}",serde_json::to_string(&v.to_serializable().detect_mixed_tables(true)).unwrap());println!("no-detect:{}",serde_json::to_string(&v.to_serializable()).unwrap()); Ok(())}
fn sets()->R{let lua=Lua::new();let s:HashSet<i32>=[1,2,3].into();let v=s.into_lua(&lua)?;println!("{:?}",lua.unpack::<HashSet<i32>>(v.clone()).map(|s|s.len()));println!("{:?}",lua.unpack::<BTreeSet<i32>>(v).map(|s|s.len()));
  let s:HashSet<i32>=[5,7].into();let v=s.into_lua(&lua)?;println!("{:?}",lua.unpack::<HashSet<i32>>(v).map(|s|s.len()));
  let v:Value=lua.load("return {'x','y',a=true}").eval()?;println!("{:?}",lua.unpack::<HashSet<String>>(v)); Ok(())}
fn clear_leak()->R{let lua=Lua::new();let t=lua.create_table()?;for i in 0..2_000_000{if let Err(e)=t.clear(){println!("i={i} {e}");return Ok(())}} println!("noleak"); Ok(())}
fn clear_iter()->R{let lua=Lua::new();let t=lua.create_table()?;for i in 0..50{t.set(format!("k{i}"),i)?;}let o=lua.create_table()?;
  t.for_each(|k:String,v:i32|{o.clear()?;println!("{k} {v}");Ok(())})?;println!("survived"); Ok(())}
fn clear_iter2()->R{let lua=Lua::new();let t:Table=lua.load("return {1,2,3,4,5}").eval()?;let o=lua.create_table()?;
  t.for_each_value(|v:i32|{o.clear()?;println!("{v}");Ok(())})?;println!("survived"); Ok(())}
fn oom()->R{let lua=Lua::new(); println!("{:?}",lua.load("return pcall(string.rep,'x',1<<30)").eval::<MultiValue>().map(|m|m.len())); Ok(())}
fn recycle_hook()->R{let lua=Lua::new_with(StdLib::ALL_SAFE,LuaOptions::new().thread_pool_size(4))?;
  let c=std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)); let c2=c.clone();
  lua.set_global_hook(HookTriggers::new().every_nth_instruction(1),move|_,_|{c2.fetch_add(1,std::sync::atomic::Ordering::SeqCst);Ok(VmState::Continue)})?;
  let f:Function=lua.load("for i=1,100 do end return 1").into_function()?;
  let w=noop_waker();let mut cx=Context::from_waker(&w);
  for r in 0..3 { c.store(0,std::sync::atomic::Ordering::SeqCst); let mut fut=Box::pin(f.call_async::<i32>(())); let p=fut.as_mut().poll(&mut cx); println!("round {r} ready={} hook fired {}",p.is_ready(),c.load(std::sync::atomic::Ordering::SeqCst)); }
  Ok(())}
fn co_global()->R{let lua=Lua::new(); lua.load("real=coroutine.yield; coroutine=nil").exec()?;
  println!("{:?}",lua.create_async_function(|_,()|async{Ok(1)}).map(|_|()).map_err(|e|e.to_string()));
  let lua=Lua::new(); lua.load("local y=coroutine.yield; coroutine.yield=function(...) hijacked=true return y(...) end").exec()?;
  let f=lua.create_async_function(|_,()|async{Ok(1)})?; lua.globals().set("f",f)?;
  let g:Function=lua.load("return f()").into_function()?; let w=noop_waker();let mut cx=Context::from_waker(&w);let mut fut=Box::pin(g.call_async::<i32>(()));println!("{:?}",fut.as_mut().poll(&mut cx).is_ready());
  println!("hijacked={:?}",lua.globals().get::<Value>("hijacked")); Ok(())}
fn reset_err()->R{let lua=Lua::new();let f:Function=lua.load("error('orig')").into_function()?;let th=lua.create_thread(f)?;println!("{:?}",th.resume::<()>(()).map_err(|e|e.to_string()));println!("st {:?}",th.status());let g=lua.create_function(|_,()|Ok(()))?;println!("reset {:?}",th.reset(g).map_err(|e|e.to_string())); Ok(())}
fn pairs_stop()->R{let lua=Lua::new();let t:Table=lua.load("local t={} for i=1,50 do t['k'..i]=i end t.k1='bad' t.k2='bad' t.k3='bad' return t").eval()?;let(mut ok,mut er)=(0,0);for p in t.pairs::<String,i32>(){if p.is_ok(){ok+=1}else{er+=1}}println!("ok={ok} err={er}");let t:Table=lua.load("return {1,'x',3}").eval()?;let v:Vec<_>=t.sequence_values::<i32>().map(|x|x.is_ok()).collect();println!("{:?}",v); Ok(())}
fn nocatch()->R{let code="local co=coroutine.wrap(function() return pcall(function() local x=coroutine.yield(1); return x end) end); local a=co(); local b,c=co(2); return a,b,c";
  let lua=Lua::new();println!("{:?}",lua.load(code).eval::<MultiValue>().map_err(|e|e.to_string()));
  let lua=Lua::new_with(StdLib::ALL_SAFE,LuaOptions::new().catch_rust_panics(false))?;println!("{:?}",lua.load(code).eval::<MultiValue>().map_err(|e|e.to_string())); Ok(())}
fn invbool()->R{let lua=Lua::new();let f:Function=lua.load("local x <close> = setmetatable({}, {__close=function() error(true) end}); coroutine.yield()").into_function()?;let th=lua.create_thread(f)?;th.resume::<()>(())?;println!("{:?}",th.reset(lua.create_function(|_,()|Ok(()))?).map_err(|e|e.to_string())); Ok(())}
fn gcfin()->R{let lua=Lua::new();let f=lua.create_function(|lua,m:i32|{println!("running={}",lua.gc_is_running()); if m==0 {lua.gc_inc(0,0,0);} else {lua.gc_gen(0,0);} Ok(())})?;lua.globals().set("f",f)?;
  println!("{:?}",lua.load("setmetatable({}, {__gc=function() print('gc',pcall(f,0)) end}); collectgarbage(); collectgarbage()").exec().map_err(|e|e.to_string())); Ok(())}
fn val_eq()->R{let lua=Lua::new();println!("{}",Value::Integer((1<<53)+1)==Value::Number((1u64<<53) as f64));println!("{:?}",lua.load("return (1<<53)+1 == 2^53").eval::<bool>()); Ok(())}
fn pretty()->R{let lua=Lua::new();let t:Value=lua.load("local s={1} return {a=s,b=s}").eval()?;println!("{:#?}",t);let t:Value=lua.load("return {[''] = 1, ['end']=2}").eval()?;println!("{:#?}",t); Ok(())}
fn destroy()->R{#[derive(Debug)]struct D(String);impl UserData for D{fn add_methods<M:UserDataMethods<Self>>(m:&mut M){m.add_function("f",|_,ud:AnyUserData|{let r=ud.borrow::<D>()?;println!("before {}",r.0.len());let e=ud.destroy();println!("destroy {:?}",e);let mut v=vec![];for _ in 0..50{v.push(vec![0x41u8;64]);}println!("after len={} ptr={:?}",r.0.len(),r.0.as_ptr());println!("{}",r.0.chars().count());std::mem::forget(v);Ok(())});
   m.add_method("g",|lua,this,ud:AnyUserData|{let e=ud.destroy();println!("destroy-from-method {:?}",e);let v=vec![0x41u8;96];println!("after len={}",this.0.len());Ok(())});
   }}
  let lua=Lua::new();let u=lua.create_userdata(D("x".repeat(64)))?;lua.globals().set("u",u)?;println!("{:?}",lua.load("u.f(u)").exec().map_err(|e|e.to_string()));
  let u=lua.create_userdata(D("y".repeat(64)))?;lua.globals().set("v",u)?;println!("{:?}",lua.load("v:g(v)").exec().map_err(|e|e.to_string())); Ok(())}
fn yield_with(m:u8)->R{let lua=Lua::new();let f=lua.create_async_function(|lua,()|async move{let a:Value=lua.yield_with::<Value>(1).await?;println!("resumed with {:?}",a);let b:MultiValue=lua.yield_with(2).await?;println!("resumed with {:?}",b);Ok(3)})?;
  let fail=lua.create_function(|_,()|->LuaResult<()>{Err(LuaError::runtime("x"))})?;
  lua.globals().set("fail",fail)?;lua.globals().set("gen",f)?;
  let co:Function=lua.load("return coroutine.wrap(gen)").eval()?;lua.globals().set("co",co)?;
  println!("{:?}",lua.load("print(co()); pcall(fail); print(co('x')); print(co('y'))").exec().map_err(|e|e.to_string())); Ok(())}
fn hook_yield(gc:bool)->R{let lua=Lua::new();let f:Function=lua.load("local a='hello'\nlocal t={1}\nlocal b=a..'x'\nlocal c=t[1]\nlocal d=b..'y'\nreturn d..c").into_function()?;let th=lua.create_thread(f)?;
  th.set_hook(HookTriggers::new().every_line(),|_,_|Ok(VmState::Yield))?;
  loop{let r=th.resume::<MultiValue>(());match r{Ok(v)=>{if th.status()==ThreadStatus::Finished{println!("done {:?}",v);break}} Err(e)=>{println!("err {e}");break}} if gc{lua.gc_collect()?;lua.gc_collect()?;}} Ok(())}
fn refs(m:u8)->R{let lua=Lua::new();let t:Table=lua.load("local t={} for i=1,1100000 do t[i]='s'..i end return t").eval()?;let v:Vec<Value>=t.sequence_values().collect::<LuaResult<_>>()?;println!("{}",v.len()); Ok(())}
fn refs_lim()->R{let lua=Lua::new();lua.set_memory_limit(512*1024)?;let mut v=vec![];for i in 0..1000000{match lua.create_string("x"){Ok(s)=>v.push(s),Err(e)=>{println!("i={i} {e}");return Ok(())}}} Ok(())}
fn bytecode()->R{let lua=Lua::new();println!("{:?}",lua.load(r#"
 local f=string.dump(function(...) local a,b,c=... return a+b+c, {a,b}, #c end)
 math.randomseed(7)
 for i=1,20000 do local p=math.random(30,#f) local s=f:sub(1,p-1)..string.char(math.random(0,255))..f:sub(p+1)
   local g=load(s,'x','b'); if g then pcall(g,1,2,3) end end return 'nocrash'"#).eval::<String>()); Ok(())}
fn manyret()->R{let lua=Lua::new();let f:Function=lua.load("local t={} for i=1,20 do t[i]='s'..i end return table.unpack(t)").into_function()?;println!("{:?}",f.call::<MultiValue>(()).map(|m|m.len())); Ok(())}
fn manyret2()->R{let lua=Lua::new();let f:Function=lua.load("local t={} for i=1,31 do t[i]=i end t[31]=u return table.unpack(t)").into_function()?;println!("{:?}",f.call::<Vec<i32>>(()).map(|m|m.len()));println!("env {:?}",f.environment().is_some());
 let f:Function=lua.load("local t={} for i=2,30 do t[i]=i end t[1]={1,2,3} return table.unpack(t)").into_function()?;println!("{:?}",f.call::<(Vec<i32>,MultiValue)>(()).map(|m|m.0.len()));Ok(())}
fn seqchk()->R{let lua=Lua::new();let f=lua.create_function(|lua,t:Table|{let v:Vec<i32>=t.sequence_values().collect::<LuaResult<_>>()?;Ok(v.len())})?;println!("{:?}",f.call::<usize>(lua.load("return {1,2,3}").eval::<Table>()?)); Ok(())}
fn fromfloat()->R{#[derive(serde::Deserialize,Debug)]struct A{a:u32}let lua=Lua::new();let v:Value=lua.load("return {a=10/2}").eval()?;println!("{:?}",lua.from_value::<A>(v).map_err(|e|e.to_string()));println!("{:?}",lua.unpack::<u32>(Value::Number(5.0))); Ok(())}
fn shebang()->R{let lua=Lua::new();let d=std::env::temp_dir();let a=d.join("sb.lua");std::fs::write(&a,"#!/usr/bin/env lua\nreturn 1\n").unwrap();let b=d.join("bom.lua");std::fs::write(&b,b"\xef\xbb\xbfreturn 2\n").unwrap();
 println!("{:?}",lua.load(a.as_path()).eval::<i32>().map_err(|e|e.to_string()));println!("{:?}",lua.load(b.as_path()).eval::<i32>().map_err(|e|e.to_string()));
 println!("dofile {:?}",lua.load(format!("return dofile('{}')",a.display())).eval::<i32>().map_err(|e|e.to_string()));println!("dofile {:?}",lua.load(format!("return dofile('{}')",b.display())).eval::<i32>().map_err(|e|e.to_string())); Ok(())}
fn equals()->R{let lua=unsafe{Lua::unsafe_new()};let t:Vec<Value>=lua.load("return io.stdout,io.stderr").eval::<MultiValue>()?.into_iter().collect();println!("{:?}",t[0].equals(&t[1]));
 struct A;impl UserData for A{}struct B;impl UserData for B{fn add_methods<M:UserDataMethods<Self>>(m:&mut M){m.add_meta_method(mlua::MetaMethod::Eq,|_,_,_:Value|Ok(true));}}
 let a=lua.create_userdata(A)?;let b=lua.create_userdata(B)?;lua.globals().set("a",a.clone())?;lua.globals().set("b",b.clone())?;println!("lua a==b {:?}",lua.load("return a==b").eval::<bool>());println!("ud eq {:?}",Value::UserData(a).equals(&Value::UserData(b)));
 let x:Table=lua.load("return setmetatable({}, {__eq=setmetatable({}, {__call=function() return true end})})").eval()?;let y=lua.create_table()?;println!("{:?}",x.equals(&y).map_err(|e|e.to_string()));println!("{:?}",lua.load("local x=setmetatable({}, {__eq=setmetatable({}, {__call=function() return true end})}) return x=={}").eval::<bool>().map_err(|e|e.to_string())); Ok(())}
fn trycache()->R{let lua=Lua::new();let m=lua.load("local x = nil + 1").set_name("=mychunk").into_function();println!("{:?}",m.map(|_|())); println!("{:?}",lua.load("local x = nil + 1").set_name("=mychunk").exec().map_err(|e|e.to_string()));
  let f=lua.create_async_function(|_,()|async{Ok(())})?; let e=lua.load("local f=...;coroutine.yield=nil; return f()").call::<()>(f); println!("{:?}",e.map_err(|e|e.to_string())); Ok(())}
fn tostr()->R{let lua=Lua::new();println!("{:?} {:?}",Value::Number(1.0).to_string(),Value::Number(1e100).to_string());println!("{:?}",lua.load("return tostring(1.0),tostring(1e100)").eval::<(String,String)>()); Ok(())}
fn dbg_after()->R{let lua=Lua::new();let t=lua.create_table()?;drop(lua);println!("{:?}",t);Ok(())}
fn getpath()->R{let lua=Lua::new();let t:Value=lua.load("return {[math.mininteger]=5,[-5]=6}").eval()?;let Value::Table(t)=t else{panic!()};println!("{:?}",t.get_path::<i32>("[-5]"));println!("{:?}",t.get_path::<i32>("[-9223372036854775808]").map_err(|e|e.to_string()));Ok(())}
fn intkeys()->R{struct G;impl UserData for G{fn add_fields<F:mlua::UserDataFields<Self>>(f:&mut F){f.add_field_method_get("a",|_,_|Ok(1));}}struct M;impl UserData for M{fn add_fields<F:mlua::UserDataFields<Self>>(f:&mut F){f.add_field_method_get("a",|_,_|Ok(1));}fn add_methods<T:UserDataMethods<Self>>(m:&mut T){m.add_method("m",|_,_,()|Ok(1));}}
 let lua=Lua::new();lua.globals().set("g",G)?;lua.globals().set("m",M)?;
 for s in ["return g[{}]","return g.zz","return m.zz","return m[{}]","g[{}]=1","return g[1]"]{println!("{s}: {:?}",lua.load(s).eval::<Value>().map_err(|e|e.to_string().lines().next().unwrap().to_string()));} Ok(())}
fn sort_cmp()->R{let lua=Lua::new();for seed in 0..40{let t:Table=lua.load(format!("math.randomseed({seed}); local t={{}} for i=1,400 do local n=math.random(0,2000); if math.random()<0.5 then t[math.maxinteger-n]=1 else t[2^63*(1-n/2^53)]=1 end end t[2^63]=1 t[2^64]=1 return t")).eval()?;
  let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||format!("{:#?}",t).len()));if r.is_err(){println!("seed {seed} panicked");return Ok(())}}println!("nopanic");Ok(())}
fn denynull()->R{let lua=Lua::new();lua.globals().set("null",lua.null())?;let v:Value=lua.load("return {1,null,3}").eval()?;println!("{}",serde_json::to_string(&v.to_serializable()).unwrap());println!("{}",serde_json::to_string(&v.to_serializable().deny_unsupported_types(false)).unwrap());
 println!("{:?}",lua.from_value_with::<Vec<Option<i32>>>(v.clone(),DeserializeOptions::new().deny_unsupported_types(false)));println!("{:?}",lua.from_value::<Vec<Option<i32>>>(v)); Ok(())}
fn hookrot()->R{let lua=Lua::new();lua.set_hook(HookTriggers::new().every_line(),|lua,_|{let f:Option<Function>=lua.globals().get("cl").ok();if let Some(f)=f{let v:i32=f.call(())?;println!("hook sees b={v}");}Ok(VmState::Continue)})?;
 let fail=lua.create_function(|_,()|->LuaResult<()>{Err(LuaError::runtime("x"))})?;lua.globals().set("fail",fail)?;
 println!("{:?}",lua.load("local a,b=111,222\ncl=function() return b end\nlocal s=0\npcall(fail)\ns=s+1\ncl=nil\nreturn b").eval::<i32>()); Ok(())}
fn pushlen()->R{let lua=Lua::new();let t:Table=lua.load("return setmetatable({}, {__len=function() return math.maxinteger end})").eval()?;println!("{:?}",t.push(1));Ok(())}
fn close_abort()->R{let lua=Lua::new();let f:Function=lua.load("local x <close> = setmetatable({}, {__close=function() error('boom') end}); local y = nil + 1").into_function()?;let th=lua.create_thread(f)?;println!("{:?}",th.resume::<()>(()).map_err(|e|e.to_string()));println!("survived");Ok(())}
fn close_gc(m:u8)->R{struct H(Lua,u8);impl Drop for H{fn drop(&mut self){println!("drop start");match self.1{0=>{let l=self.0.clone();println!("cloned");drop(l);},_=>{println!("{:?}",self.0.globals().set("a",1));}}println!("drop end");}}
  let lua=Lua::new();let weak=lua.weak();let f=lua.create_function(move|lua,()|{println!("gc fn");match m{0=>{let l=lua.clone();println!("cloned ok");drop(l);},1=>{println!("{:?}",lua.app_data_ref::<i32>().is_some());},_=>{println!("{:?}",lua.globals().set("a",1));}}println!("gc fn end");Ok(())})?;lua.globals().set("f",f)?;lua.load("x=setmetatable({}, {__gc=function() f() end})").exec()?;drop(lua);println!("dropped");Ok(())}
fn tuplenone()->R{#[derive(serde::Serialize)]enum E{T(Option<i32>,i32)}#[derive(serde::Serialize)]struct TS(Option<i32>,i32);let lua=Lua::new();let o=SerializeOptions::new().serialize_none_to_null(false);let v=lua.to_value_with(&E::T(None,7),o)?;let Value::Table(t)=v else{panic!()};let i:Table=t.get("T")?;println!("variant t[1]={:?} t[2]={:?}",i.get::<Value>(1),i.get::<Value>(2));
 let v=lua.to_value_with(&TS(None,7),o)?;let Value::Table(t)=v else{panic!()};println!("struct t[1]={:?} t[2]={:?}",t.get::<Value>(1),t.get::<Value>(2));let v=lua.to_value_with(&(None::<i32>,7),o)?;let Value::Table(t)=v else{panic!()};println!("tuple t[1]={:?} t[2]={:?}",t.get::<Value>(1),t.get::<Value>(2));
 let v=lua.to_value_with(&vec![None,Some(7)],o)?;let Value::Table(t)=v else{panic!()};println!("seq t[1]={:?} t[2]={:?}",t.get::<Value>(1),t.get::<Value>(2)); Ok(())}
fn leak()->R{let lua=Lua::new();let rc=Rc::new(1);let rc2=rc.clone();let u=lua.used_memory();lua.set_memory_limit(u+1)?;
  let s=unsafe{lua.create_string("x")};println!("str {:?}",s.is_ok());
  let g=std::rc::Rc::new(());let r=lua.create_function({let rc2=rc2.clone();move|_,()|{let _=&rc2;Ok(())}});println!("fn {:?}",r.as_ref().map(|_|()).map_err(|e|e.to_string()));drop(r);drop(rc2);lua.set_memory_limit(0)?;lua.gc_collect()?;lua.gc_collect()?;println!("strong={}",Rc::strong_count(&rc)); Ok(())}
fn seqhint()->R{let lua=Lua::new();lua.globals().set("pr",lua.create_function(|_,()|Ok(()))?)?;let v:Value=lua.load("return {1,pr,3}").eval()?;println!("{}",serde_json::to_string(&v.to_serializable().deny_unsupported_types(false)).unwrap());Ok(())}
fn set_nil_ctx()->R{Ok(())}

fn main(){let a=std::env::args().nth(1).unwrap();
 let r=match a.as_str(){
 "scope_uaf"=>scope_uaf(),"scope_uaf_any"=>scope_uaf_any(),"foreign_mt"=>foreign_mt(),"foreign_thread"=>foreign_thread(),"stale"=>stale(),
 "nil_set"=>nil_set(),"nan_rawset"=>nan_rawset(),"nil_rawremove"=>nil_rawremove(),"nil_from"=>nil_from(),"nil_tovalue"=>nil_tovalue(),"nil_limit"=>nil_limit(),"nil_cb"=>nil_cb(),
 "mut_pairs"=>mut_pairs(),"mut_foreach"=>mut_foreach(),"err_key0"=>err_key(0),"err_key1"=>err_key(1),"err_key2"=>err_key(2),
 "deep0"=>deep(0,200000),"deep1"=>deep(1,200000),"deep2"=>deep(2,200000),"deep0s"=>deep(0,5000),
 "drop_panic"=>drop_panic(),"warn_err"=>warn_err(),"warn_panic"=>warn_panic(),"warn_co"=>warn_co(),
 "normal0"=>normal_status(0),"normal1"=>normal_status(1),"normal2"=>normal_status(2),
 "call_async_drop"=>call_async_drop(false),"call_async_hang"=>call_async_drop(true),"eqslice"=>eqslice(),"sparse"=>sparse(),"sets"=>sets(),
 "clear_leak"=>clear_leak(),"clear_iter"=>clear_iter(),"clear_iter2"=>clear_iter2(),"oom"=>oom(),"recycle_hook"=>recycle_hook(),"co_global"=>co_global(),
 "reset_err"=>reset_err(),"pairs_stop"=>pairs_stop(),"nocatch"=>nocatch(),"invbool"=>invbool(),"gcfin"=>gcfin(),"val_eq"=>val_eq(),"pretty"=>pretty(),
 "destroy"=>destroy(),"yield_with"=>yield_with(0),"hook_yield0"=>hook_yield(false),"hook_yield1"=>hook_yield(true),"refs"=>refs(0),"refs_lim"=>refs_lim(),"bytecode"=>bytecode(),
 "manyret"=>manyret(),"manyret2"=>manyret2(),"fromfloat"=>fromfloat(),"shebang"=>shebang(),"equals"=>equals(),"trycache"=>trycache(),"tostr"=>tostr(),"dbg_after"=>dbg_after(),
 "getpath"=>getpath(),"intkeys"=>intkeys(),"sort_cmp"=>sort_cmp(),"denynull"=>denynull(),"hookrot"=>hookrot(),"pushlen"=>pushlen(),"close_abort"=>close_abort(),
 "close_gc0"=>close_gc(0),"close_gc1"=>close_gc(1),"close_gc2"=>close_gc(2),"tuplenone"=>tuplenone(),"leak"=>leak(),"seqhint"=>seqhint(),
 _=>panic!("unknown")};
 println!("result: {:?}",r.map_err(|e|e.to_string()));
}
