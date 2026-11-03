use simd_json::prelude::*;
#[derive(serde::Deserialize,Debug)] #[allow(dead_code)] struct R{sensor:String,time:u64,unit:String,lat:f64}
fn main(){ let doc=r#"{"sensor":"a1","time":1700000000,"unit":"degreesN","lat":48.858369400000000000012}"#; println!("{doc}");
 let mut b=doc.as_bytes().to_vec(); println!("owned lat={:?}",simd_json::to_owned_value(&mut b).unwrap().get("lat").unwrap().as_f64());
 let mut b=doc.as_bytes().to_vec(); println!("struct {:?}",simd_json::from_slice::<R>(&mut b));
 println!("serde_json {:?}",serde_json::from_str::<serde_json::Value>(doc).unwrap()["lat"]);
}
