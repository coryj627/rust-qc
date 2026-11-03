use simd_json::prelude::*;
fn main(){
    for d in [r#"{"id":12345,"name":"sensor-abcdefghijklmnopqrstuvw","reading":3.14159265358979323846264338}"#, r#"{"id":12345,"name":"sensor-abcdefghijklmnopqrstuvw","big":777253035878364813707596e287}"#] {
        let mut b=d.as_bytes().to_vec();
        println!("IN {d}\nOUT {:?}\nSJ {:?}", simd_json::to_owned_value(&mut b).map(|v| v.encode()).map_err(|e|e.to_string()), serde_json::from_str::<serde_json::Value>(d).map(|v| v.to_string()).map_err(|e|e.to_string()));
    }
}
