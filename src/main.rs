extern crate clap;

use std::ops::Range;

use clap::{App, Arg};

// https://github.com/python/cpython/blob/80017752ba938852d53f9d83a404b4ecd9ff2baa/Objects/tupleobject.c#L39
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
        let base = self.range.end - self.range.start;
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

fn middle_lane_from_second(hash_last: u64, second_counter: &RangeCounter) -> u64 {
    let mut acc = hash_last;

    for i in second_counter.counter.iter().rev() {
        acc = acc.wrapping_mul(PYHASH_XXPRIME_1_INV);
        acc = PYHASH_XXROTATE_REV(acc);
        acc = acc.wrapping_sub(i.wrapping_mul(PYHASH_XXPRIME_2));
    }
    acc = acc.wrapping_mul(PYHASH_XXPRIME_1_INV);
    acc = PYHASH_XXROTATE_REV(acc);
    acc.wrapping_mul(PYHASH_XXPRIME_2_INV)
}

fn middle_lane_from_first(first_counter: &RangeCounter) -> u64 {
    let mut acc = PYHASH_XXPRIME_5;
    for i in first_counter.counter.iter() {
        acc = acc.wrapping_add(i.wrapping_mul(PYHASH_XXPRIME_2));
        acc = PYHASH_XXROTATE(acc);
        acc = acc.wrapping_mul(PYHASH_XXPRIME_1);
    }
    acc.wrapping_mul(PYHASH_XXPRIME_2_INV)
}

fn tuple_unhash<F>(length: usize, hash: u64, item_range: &Range<u64>, table_size: usize, printer: F)
    where F: Fn(&Vec<u64>) {
    let first_half_length = (length - 1) / 2;
    let second_half_length = length - first_half_length - 1;

    let mut second_counter = RangeCounter::new(item_range, second_half_length);

    let hash_last = hash.wrapping_sub(length as u64 ^ PYHASH_XXPRIME_5 ^ 3527539);

    let mut completed = false;

    while !completed {
        let mut table = Vec::with_capacity(table_size);

        for _ in 0..table_size {
            let lane = middle_lane_from_second(hash_last, &second_counter);

            table.push((lane, second_counter.to_index()));

            if second_counter.next() {
                completed = true;
                break;
            }
        }

        table.sort_unstable();

        let mut first_counter = RangeCounter::new(item_range, first_half_length);
        loop {
            let acc = middle_lane_from_first(&first_counter);

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

                printer(&result);

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

fn raw_printer(result: &Vec<u64>) {
    println!("{}", result.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(" "));
}

fn format_printer(result: &Vec<u64>) {
    println!("Tuple: {},\t Hash: {}", result.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(" "),
             tuple_hash(result));
}


fn main() {
    let args = App::new("Tuple-Hash-Crasher")
        .version("0.1.0")
        .about("Generate tuple with same hash")
        .arg(Arg::new("length")
            .short('l')
            .long("length")
            .takes_value(true)
            .help("Length of tuple")
            .required(true))
        .arg(Arg::new("hash")
            .short('h')
            .long("hash")
            .takes_value(true)
            .help("Hash of tuple"))
        .arg(Arg::new("item_minimum")
            .short('m')
            .long("item_minimum")
            .takes_value(true)
            .help("Minimum value of item")
            .required(true))
        .arg(Arg::new("item_maximum")
            .short('M')
            .long("item_maximum")
            .takes_value(true)
            .help("Maximum value of item")
            .required(true))
        .arg(Arg::new("table_size")
            .short('t')
            .long("table_size")
            .takes_value(true)
            .help("Table size to run Algorithm")
            .required(true))
        .arg(Arg::new("format")
            .short('f')
            .long("format")
            .help("Format tuple"));

    let matches = args.get_matches();

    let length = matches.value_of("length").unwrap().parse::<usize>().unwrap();
    let hash = if let Some(hash) = matches.value_of("hash") {
        hash.parse::<u64>().unwrap()
    } else {
        0
    };
    let item_minimum = matches.value_of("item_minimum").unwrap().parse::<u64>().unwrap();
    let item_maximum = matches.value_of("item_maximum").unwrap().parse::<u64>().unwrap();
    let table_size = matches.value_of("table_size").unwrap().parse::<usize>().unwrap();
    let format = matches.is_present("format");

    let item_range = item_minimum..item_maximum;

    if format {
        tuple_unhash(length, hash, &item_range, table_size, &format_printer);
    } else {
        tuple_unhash(length, hash, &item_range, table_size, &raw_printer);
    }
}
