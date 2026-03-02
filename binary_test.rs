use std::rc::Rc;
use std::mem::size_of;

#[derive(Clone)]
enum ValueVec {
    Binary(Vec<u8>),
}

#[derive(Clone)]
enum ValueRc {
    Binary(Rc<Vec<u8>>),
}

fn main() {
    let mut vec_sizes = Vec::new();
    let mut rc_sizes = Vec::new();

    // Create large vectors
    for i in 0..5 {
        let v = vec![i as u8; 10 * 1024 * 1024]; // 10MB
        vec_sizes.push(ValueVec::Binary(v.clone()));
        rc_sizes.push(ValueRc::Binary(Rc::new(v)));
    }

    // Time cloning Vec
    let start_vec = std::time::Instant::now();
    let mut vec_clones = Vec::new();
    for v in &vec_sizes {
        vec_clones.push(v.clone());
    }
    let dur_vec = start_vec.elapsed();

    // Time cloning Rc
    let start_rc = std::time::Instant::now();
    let mut rc_clones = Vec::new();
    for r in &rc_sizes {
        rc_clones.push(r.clone());
    }
    let dur_rc = start_rc.elapsed();

    println!("Time to clone 5x 10MB Vec<u8>: {:?}", dur_vec);
    println!("Time to clone 5x 10MB Rc<Vec<u8>>: {:?}", dur_rc);
}
