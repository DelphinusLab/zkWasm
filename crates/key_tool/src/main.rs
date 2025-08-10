mod file_util;
use crate::file_util::read_zk_sync_key;
use crate::file_util::write_halo2_params;
use num_traits::Num;
use pairing_bn256::arithmetic::CurveAffine;
use pairing_bn256::bn256::Fq;
use pairing_bn256::bn256::Fq2;
use pairing_bn256::bn256::G1Affine;
use pairing_bn256::bn256::G2Affine;
use pairing_bn256::group::ff::PrimeField;
use pairing_bn256::group::GroupEncoding;
use pairing_ce::bn256::Fq as FqCE;
use pairing_ce::bn256::G2Affine as G2AffineCE;
use pairing_ce::CurveAffine as CurveAffineCE;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process;
use std::thread;

// Convert pairing_ce::G2Affine to pairing::G2Affine.
fn trans_g2(g2: G2AffineCE) -> G2Affine {
    let (x_ce, y_ce) = g2.as_xy();

    let mut x = Fq2::default();
    x.c0 = trans_fq(x_ce.c0);
    x.c1 = trans_fq(x_ce.c1);
    let mut y = Fq2::default();
    y.c0 = trans_fq(y_ce.c0);
    y.c1 = trans_fq(y_ce.c1);
    return G2Affine::from_xy(x, y).unwrap();
}

// Convert pairing_ce::Fq to pairing::Fq.
fn trans_fq(x: FqCE) -> Fq {
    let pp = Fq::from_str_vartime(&*extract_decimal_from_string(&x.to_string())).unwrap();
    return pp;
}

// Convert fq to a positive decimal.
// input="Fq(0x24fc1e1c263a7de7abec5edaeea87625890c96a018bb8c60522333fa206f70c3)"
// output=16728715820616582450594109459208172618408974327542441440317506932429837791427
fn extract_decimal_from_string(s: &str) -> String {
    let hex_str = &s[5..s.len() - 1];
    let tt = num_bigint::BigUint::from_str_radix(hex_str, 16)
        .unwrap()
        .to_string();
    return tt;
}

fn check_file_exist(monomial_key_file: String, lagrange_key_file: String) -> bool {
    if !Path::new(&monomial_key_file).exists() {
        println!("monomial_key_file not exist {:?}", monomial_key_file);
        return false;
    }
    if !Path::new(&lagrange_key_file).exists() {
        println!("lagrange_key_file not exist {:?}", lagrange_key_file);
        return false;
    }
    return true;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: cargo run -- <path of monomial key> <path of lagrange key> <path for halo2 params file>");
        process::exit(1);
    }

    let monomial_key_file = &args[1];
    let lagrange_key_file = &args[2];
    let output_halo2_key_file = &args[3];
    println!("monomial_key_file={:?}", monomial_key_file);
    println!("lagrange_key_file={:?}", lagrange_key_file);
    println!("output_halo2_key_file={:?}", output_halo2_key_file);

    if !check_file_exist(monomial_key_file.clone(), lagrange_key_file.clone()) {
        process::exit(1);
    }

    let mut buf_reader_lagrange =
        BufReader::with_capacity(1 << 29, File::open(lagrange_key_file).unwrap());
    let (lagrange_key, g2_base) = read_zk_sync_key(&mut buf_reader_lagrange).unwrap();

    let mut buf_reader_monomial =
        BufReader::with_capacity(1 << 29, File::open(monomial_key_file).unwrap());
    let (monomial_key, _) = read_zk_sync_key(&mut buf_reader_monomial).unwrap();

    let handle_lagrange = thread::spawn(move || {
        let mut g_lagrange = Vec::new();
        for index in 0..lagrange_key.len() {
            let (x, y) = lagrange_key[index].as_xy();
            g_lagrange.push(G1Affine::from_xy(trans_fq(*x), trans_fq(*y)).unwrap());
        }
        return g_lagrange;
    });

    let handle_normal = thread::spawn(move || {
        let mut g = Vec::new();
        for index in 0..monomial_key.len() {
            let (x, y) = monomial_key[index].as_xy();
            g.push(G1Affine::from_xy(trans_fq(*x), trans_fq(*y)).unwrap());
        }
        return g;
    });
    println!(
        "finish read zksync keys monomial={:?} lagrange{:?}",
        monomial_key_file, lagrange_key_file
    );

    let g_lagrange = handle_lagrange.join().unwrap();
    let g_monomial = handle_normal.join().unwrap();
    let additional_data = trans_g2(g2_base[1]).to_bytes().as_ref().to_vec();
    let k = (g_lagrange.len() as f64).log2() as u32;

    let mut fd = File::create(output_halo2_key_file).unwrap();
    write_halo2_params(&mut fd, k, g_monomial, g_lagrange, additional_data)
        .expect("write halo2 to file failed");
    println!(
        "finish write halo2 params k={:?} output_halo2_key_file={:?}",
        k, output_halo2_key_file
    )
}
