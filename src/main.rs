use std::ops::Range;
use std::ptr::hash;

const PYHASH_XXPRIME_1: u64 = 11400714785074694791;
const PYHASH_XXPRIME_2: u64 = 14029467366897019727;
const PYHASH_XXPRIME_5: u64 = 2870177450012600261;
const PYHASH_XXPRIME_1_INV: u64 = inverse_mod(PYHASH_XXPRIME_1);
const PYHASH_XXPRIME_2_INV: u64 = inverse_mod(PYHASH_XXPRIME_2);

const fn inverse_mod(a: u64) -> u64 {
    let mut s: u64 = 0;
    let mut x = 0;
    let mut i = 0;
    while i < 64 {
        if ((s ^ 1) >> i) & 1 == 1 {
            s = s.wrapping_add(a << i);
            x |= 1 << i;
        }
        i += 1;
    }
    x
}

fn PYHASH_XXROTATE(x: u64) -> u64 {
    (x << 31) | (x >> 33)
}

fn PYHASH_XXROTATE_REV(x: u64) -> u64 {
    (x >> 31) | (x << 33)
}

fn tuple_hash(item: &Vec<u64>) -> u64 {
    let length = item.len() as u64;

    let mut acc = PYHASH_XXPRIME_5;
    for i in item.iter() {
        acc = acc.wrapping_add(i.wrapping_mul(PYHASH_XXPRIME_2));
        acc = PYHASH_XXROTATE(acc);
        acc = acc.wrapping_mul(PYHASH_XXPRIME_1);
    }

    acc.wrapping_add(length ^ PYHASH_XXPRIME_5 ^ 3527539)
}

struct RangeCounter<'a> {
    n: usize,
    range: &'a Range<u64>,
    counter: Vec<u64>,
}

impl<'a> RangeCounter<'a> {
    fn new(range: &'a Range<u64>, n: usize) -> RangeCounter {
        RangeCounter {
            n,
            range,
            counter: vec![0; n],
        }
    }

    fn next(&mut self) -> bool {
        self.counter[0] += 1;
        for i in 0..self.n {
            if self.counter[i] == self.range.end {
                self.counter[i] = self.range.start;
                if i + 1 < self.counter.len() {
                    self.counter[i + 1] += 1;
                } else {
                    return true;
                }
            }
        }
        false
    }

    fn to_index(&self) -> u64 {
        let base = self.range.end - self.range.start;
        let mut index: u64 = 0;
        for i in self.counter.iter() {
            index = index.wrapping_mul(base).wrapping_add(i - self.range.start);
        }
        index
    }

    fn from_index(&self, index: u64) -> Vec<u64> {
        let mut index = index;
        let mut result = vec![0; self.n];
        let mut base = self.range.end - self.range.start;
        for i in (0..self.n).rev() {
            result[i] = index % base + self.range.start;
            index /= base;
        }
        result
    }
}

fn lower_bound(vector: &Vec<(u64, u64)>, item: u64) -> usize {
    let mut low = 0;
    let mut high = vector.len();
    while low < high {
        let mid = (low + high) / 2;
        if vector[mid].0 < item {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

fn upper_bound(vector: &Vec<(u64, u64)>, item: u64) -> usize {
    let mut low = 0;
    let mut high = vector.len();
    while low < high {
        let mid = (low + high) / 2;
        if vector[mid].0 <= item {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

fn tuple_unhash(length: usize, hash: u64, item_range: &Range<u64>, table_size: usize) {
    let first_half_length = (length - 1) / 2;
    let second_half_length = length - first_half_length - 1;

    let mut second_counter = RangeCounter::new(item_range, second_half_length);

    let hash_last = hash.wrapping_sub(length as u64 ^ PYHASH_XXPRIME_5 ^ 3527539);

    let mut completed = false;

    while !completed {
        let mut table = Vec::with_capacity(table_size);

        for _ in 0..table_size {
            let mut acc = hash_last;

            for i in second_counter.counter.iter().rev() {
                acc = acc.wrapping_mul(PYHASH_XXPRIME_1_INV);
                acc = PYHASH_XXROTATE_REV(acc);
                acc = acc.wrapping_sub(i.wrapping_mul(PYHASH_XXPRIME_2));
            }
            acc = acc.wrapping_mul(PYHASH_XXPRIME_1_INV);
            acc = PYHASH_XXROTATE_REV(acc);
            let lane = acc.wrapping_mul(PYHASH_XXPRIME_2_INV);

            table.push((lane, second_counter.to_index()));

            if second_counter.next() {
                completed = true;
                break;
            }
        }

        table.sort();

        let mut first_counter = RangeCounter::new(item_range, first_half_length);
        loop {
            let mut acc = PYHASH_XXPRIME_5;
            for i in first_counter.counter.iter() {
                acc = acc.wrapping_add(i.wrapping_mul(PYHASH_XXPRIME_2));
                acc = PYHASH_XXROTATE(acc);
                acc = acc.wrapping_mul(PYHASH_XXPRIME_1);
            }
            acc = acc.wrapping_mul(PYHASH_XXPRIME_2_INV);

            let lower_index = lower_bound(&table, acc.wrapping_add(item_range.start));
            let upper_index = upper_bound(&table, acc.wrapping_add(item_range.end - 1));

            let mut index = lower_index;
            loop {
                if index == upper_index {
                    break;
                }
                let (lane, counter_index) = table[index];
                let mut result = vec![0; length];

                result[0..first_half_length].clone_from_slice(&first_counter.counter);
                result[first_half_length] = lane.wrapping_sub(acc);
                result[first_half_length + 1..].clone_from_slice(&second_counter.from_index(counter_index));

                println!("{:?}", result);
                println!("{}", tuple_hash(&result));

                index += 1;
                if index == table.len() {
                    index = 0;
                }
            }

            if first_counter.next() {
                break;
            }
        }
    }
}


fn main() {
    tuple_unhash(5, 1234567, &(0..200_000), 10_000_000);
}