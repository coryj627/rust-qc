// d38a2da: Buf deref passes a null pointer to slice::from_raw_parts. Safe code only.
use git2::Buf;
use std::hint::black_box;

fn main() {
    let buf = Buf::new();
    println!("Buf::new() returned, dereferencing");
    // Debug: std's precondition check aborts inside this deref.
    let r: &[u8] = &buf;
    // Release: the check is compiled out and a &[u8] holding a null data
    // pointer escapes. Option<&[u8]> uses null as its None niche, so wrapping
    // the reference in Some and reading it back yields None.
    let wrapped: Option<&[u8]> = black_box(Some(r));
    // In release builds expect ptr_is_null=false next to is_none()=true. rustc tells LLVM that
    // a reference is never null, so the is_null test on it is folded to false even though the
    // same value reads back as the None niche. That contradiction is the UB showing through.
    println!(
        "ptr_is_null={} Some(&*buf).is_none()={}",
        black_box(r).as_ptr().is_null(),
        wrapped.is_none()
    );
}
