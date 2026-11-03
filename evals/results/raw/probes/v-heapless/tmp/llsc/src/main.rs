use heapless::box_pool;
box_pool!(P: u32);
fn main(){ println!("{:?}", P.alloc(1).is_ok()); }
