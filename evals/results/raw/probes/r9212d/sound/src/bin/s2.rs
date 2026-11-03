fn main(){ let t=simd_json::Tape::null().reset(); println!("{:?}",t.deserialize::<u8>().map_err(|e|e.to_string())); }
