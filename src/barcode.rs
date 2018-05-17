// Copyright (c) 2018 10x Genomics, Inc. All rights reserved.

//! Types for dealing with 10x barcodes

use std::cmp::max;
use ordered_float::NotNaN;
use failure::Error;
use std::path::Path;

use std::io::BufRead;
use fxhash::{FxHashSet, FxHashMap};
use std::hash::Hash;

use {Barcode, SSeq};
use utils;


/// Load a (possibly gzipped) barcode whitelist file.
/// Each line in the file is a single whitelist barcode.
/// Barcodes are numbers starting at 1.
fn load_barcode_whitelist(filename: impl AsRef<Path>) -> Result<FxHashMap<SSeq, u32>, Error> {

    let reader = utils::open_with_gz(filename)?;
    let mut bc_map = FxHashMap::default();

    let mut i = 1u32;
    for l in reader.lines() {
        let seq = SSeq::new(&l?.into_bytes());
        bc_map.insert(seq, i);
        i += 1;
    }

    Ok(bc_map)
}

pub(crate) fn reduce_counts<K: Hash + Eq>(mut v1: FxHashMap<K, u32>, mut v2: FxHashMap<K, u32>) -> FxHashMap<K, u32> {
    for (k,v) in v2.drain() {
        let counter = v1.entry(k).or_insert(0);
        *counter += v;
    }

    v1
}

pub(crate) fn reduce_counts_err<K: Hash + Eq>(v1: Result<FxHashMap<K, u32>, Error>, v2: Result<FxHashMap<K, u32>, Error>) -> Result<FxHashMap<K, u32>, Error> {
    match (v1, v2) {
        (Ok(m1), Ok(m2)) => Ok(reduce_counts(m1, m2)),
        (Err(e1), _) => Err(e1),
        (_, Err(e2)) => Err(e2), 
    }
}



pub struct BarcodeChecker {
    whitelist: FxHashSet<SSeq>
}

impl BarcodeChecker { 
    pub fn new(whitelist: impl AsRef<Path>) -> Result<BarcodeChecker, Error> {
        let wl = load_barcode_whitelist(whitelist)?;

        let mut whitelist = FxHashSet::default();
        whitelist.extend(wl.keys());
        Ok(BarcodeChecker { whitelist })
    }

    pub fn check(&self, barcode: &mut Barcode) {
        if self.whitelist.contains(&barcode.sequence) {
            barcode.valid = true;
        } else {
            barcode.valid = false;
        }
    }
}

pub fn load_barcode_counts(count_files: &[impl AsRef<Path>]) -> Result<FxHashMap<Barcode, u32>, Error> {

    let mut counts = FxHashMap::default();
    for f in count_files {
        let shard_counts = ::utils::read_obj(f.as_ref())?;
        counts = reduce_counts(counts, shard_counts);
    }

    Ok(counts)
}


const BASE_OPTS: [u8; 4] = [b'A', b'C', b'G', b'T'];
type Of64 = NotNaN<f64>;

pub struct BarcodeCorrector {
	whitelist: FxHashSet<SSeq>,
    bc_counts: FxHashMap<Barcode, u32>,
	max_expected_barcode_errors: f64,
	bc_confidence_threshold: f64,
}

impl BarcodeCorrector {

    /// Load a barcode corrector from a whitelist path
    /// and a set of barcode count files.
    /// # Arguments
    /// * `whitelist` - path the barcode whitelist file
    /// * `count_files` - paths to barcode count files written by sort step
    /// * `maximum_expected_barcode_errors` - threshold for sum of probability
    ///   of error on barcode QVs. Reads exceeding this threshold will be marked
    ///   as not valid.
    /// * `bc_confidence_threshold` - if the posterior probability of a correction
    ///    exceeds this threshold, the barcode will be corrected.
    pub fn new(
        whitelist: impl AsRef<Path>, 
        count_files: &[impl AsRef<Path>],
        max_expected_barcode_errors: f64,
        bc_confidence_threshold: f64
        ) -> Result<BarcodeCorrector, Error> {

        // load whitelist into Set
        let wl = load_barcode_whitelist(whitelist)?;
        let mut whitelist = FxHashSet::default();
        whitelist.extend(wl.keys());

        let counts = load_barcode_counts(count_files)?;

        Ok(BarcodeCorrector {
            whitelist: whitelist,
            bc_counts: counts,
            max_expected_barcode_errors,
            bc_confidence_threshold
        })
    }


    pub fn correct_barcode(&self, orig_barcode: &Barcode, qual: &[u8]) -> Option<Barcode> {

        let mut a = orig_barcode.sequence.clone();

        let mut candidates: Vec<(Of64, Barcode)> = Vec::new();
        let mut total_likelihood = Of64::from(0.0);
         
	    for pos in 0 .. orig_barcode.sequence.len() {
            let qv = qual[pos];
            let existing = a.sequence[pos];
            for val in BASE_OPTS.iter() {

                if *val == existing { continue; }
                a.sequence[pos] = *val;
                let trial_bc = Barcode { valid: true, sequence: a, gem_group: orig_barcode.gem_group };

                if self.whitelist.contains(&a) {
                    let bc_count = self.bc_counts.get(&trial_bc).cloned().unwrap_or(0);
                    let prob_edit = max(Of64::from(0.0005), Of64::from(probability(qv)));
                    let likelihood = prob_edit * max(Of64::from(bc_count as f64), Of64::from(0.5));
                    candidates.push((likelihood, trial_bc));
                    total_likelihood += likelihood;
                }
            }
            a.sequence[pos] = existing;
        }
	
        let thresh = Of64::from(self.bc_confidence_threshold);
        //println!("cands: {:?}", candidates);
        let best_option = candidates.into_iter().max();
        

        let expected_errors: f64 = qual.iter().cloned().map(probability).sum();

        match best_option {
            Some((best_like, best_bc)) => {
                if expected_errors < self.max_expected_barcode_errors && best_like / total_likelihood > thresh {
                    return Some(best_bc);
                }
            }
            _ => (),
        };

        return None;
    }
}


pub fn probability(qual: u8) -> f64 {
    //33 is the illumina qual offset
    let q = qual as f64;
    (10_f64).powf(-(q - 33.0) / 10.0) 
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_barcode_correction()
    {
        let mut wl = FxHashSet::default();

        let b1 = Barcode::test_valid(b"AAAAA");
        let b2 = Barcode::test_valid(b"AAGAC");
        let b3 = Barcode::test_valid(b"ACGAA");
        let b4 = Barcode::test_valid(b"ACGTT");

        wl.insert(b1.sequence);
        wl.insert(b2.sequence);
        wl.insert(b3.sequence);
        wl.insert(b4.sequence);

        let mut counts = FxHashMap::default();
        counts.insert(b1, 100);
        counts.insert(b2, 11);
        counts.insert(b3, 2);

        let val = BarcodeCorrector {
            max_expected_barcode_errors: 1.0,
            bc_confidence_threshold: 0.95,
            whitelist: wl,
            bc_counts: counts,
        };

        // Easy
        let t1 = Barcode::test_invalid(b"AAAAA");
        //assert_eq!(val.correct_barcode(&t1, &vec![66,66,66,66,66]), Some(Barcode::test_valid(b"AAAAA")));

        // Low quality
        assert_eq!(val.correct_barcode(&t1, &vec![34,34,34,66,66]), None);

        // Trivial correction
        let t2 = Barcode::test_invalid(b"AAAAT");
        assert_eq!(val.correct_barcode(&t2, &vec![66,66,66,66,40]), Some(b1));

        // Pseudo-count kills you
        let t3 = Barcode::test_invalid(b"ACGAT");
        assert_eq!(val.correct_barcode(&t3, &vec![66,66,66,66,66]), None);

        // Quality help you
        let t4 = Barcode::test_invalid(b"ACGAT");
        assert_eq!(val.correct_barcode(&t4, &vec![66,66,66,66,40]), Some(b3));

        // Counts help you
        let t5 = Barcode::test_invalid(b"ACAAA");
        assert_eq!(val.correct_barcode(&t5, &vec![66,66,66,66,40]), Some(b1));
    }
}