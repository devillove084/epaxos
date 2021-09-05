use bitvec::prelude::BitVec;

const K2: u64 = 0x9ae16a3b2f90404f;

pub struct BloomFilter {
    pub bv: BitVec,
    pub m: u32,
    pub k: u32,
    pub powto: u32,
    pub mask: u32,
}

pub fn new_pow_two(pt: u32, k: u32) -> &'static BloomFilter {
    let m = u32::pow(2, pt);
    &BloomFilter {
        bv: BitVec::new(),
        m,
        k,
        powto: pt,
        mask: (1 << pt) - 1,
    }
}

fn hash_x(h1: u32, h2: u32, i: u32) -> u32 {
    match i {
        0 => return h1,
        1 => return h2,
        2 => return (h1 << 16) | (h2 >> 16),
        3 => return (h1 >> 16) | (h2 << 16),
        4 => return h1 + h2,
        5 => return h1 + 7 * h2,
    }
}

pub fn hash64(s: u64) -> u64 {
    let mut mul: u64 = K2 + 8;
    let mut u: u64 = 4 + (s << 3);
    let mut a: u64 = (u ^ s) * mul;
    a ^= a >> 47;
    let mut b = (s ^ a) * mul;
    b ^= b >> 47;
    b *= mul;
    return b;
}

pub fn city_hash64(s: u64) -> u64 {
    let mut mul: u64 = K2 + 16;
    let mut a = s + K2;
    let mut u = (s << 37) | (s >> 27) * mul + a;
    let mut v = ((a << 25 | a >> 39) + s) * mul;

    a = (u ^ v) * mul;
    a ^= a >> 47;
    let mut b = (v ^ a) * mul;
    b ^= b >> 47;
    b *= mul;
    return b;
}

impl BloomFilter {
    pub fn add_u64(&self, item: u64) {
        // let h64 = city_hash64(item);
        // let h32 = (h64 >> 32) as u32;
        // let l32 = h64 as u32;
        // for i in 0..self.k {
        //     let value = hash_x(l32, h32, i) & self.mask;
        //     self.bv.set(index, value)
        // }
    }
    pub fn check_u64(&self, item: u64) -> bool {
        true
    }
}

// func (bf Bloomfilter) AddUint64(item uint64) {
// 	//    h64 := hash64(item)
// 	h64 := CityHash64(item)
// 	l32 := uint32(h64)
// 	h32 := uint32(h64 >> 32)

// 	for i := uint32(0); i < bf.k; i++ {
// 		bf.bv.SetBit(hashX(l32, h32, i) & bf.mask)
// 	}
// }

// func (bf Bloomfilter) CheckUint64(item uint64) bool {
// 	//    h64 := hash64(item)
// 	h64 := CityHash64(item)
// 	l32 := uint32(h64)
// 	h32 := uint32(h64 >> 32)

// 	for i := uint32(0); i < bf.k; i++ {
// 		if !bf.bv.GetBit(hashX(l32, h32, i) & bf.mask) {
// 			return false
// 		}
// 	}
// 	return true
// }
