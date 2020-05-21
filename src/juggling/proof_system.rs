#![allow(non_snake_case)]
/*
centipede

Copyright 2018 by Kzen Networks

This file is part of centipede library
(https://github.com/KZen-networks/centipede)

Escrow-recovery is free software: you can redistribute
it and/or modify it under the terms of the GNU General Public
License as published by the Free Software Foundation, either
version 3 of the License, or (at your option) any later version.

@license GPL-3.0+ <https://github.com/KZen-networks/centipede/blob/master/LICENSE>
*/

use curv::{FE,GE,BigInt};
use curv::cryptographic_primitives::proofs::sigma_correct_homomorphic_elgamal_encryption_of_dlog::{HomoELGamalDlogProof,HomoElGamalDlogWitness,HomoElGamalDlogStatement};
use curv::cryptographic_primitives::proofs::sigma_correct_homomorphic_elgamal_enc::{HomoELGamalProof,HomoElGamalWitness,HomoElGamalStatement};
use curv::cryptographic_primitives::hashing::hash_sha512::HSha512;
use curv::cryptographic_primitives::hashing::traits::*;
use curv::arithmetic::traits::Converter;
use bulletproof::proofs::range_proof::{RangeProof,generate_random_point};
use juggling::segmentation::Msegmentation;
use Errors::{self, ErrorProving};

#[derive(Serialize, Deserialize)]
pub struct Helgamal {
    pub D: GE,
    pub E: GE,
}

#[derive(Serialize, Deserialize)]
pub struct Helgamalsegmented {
    pub DE: Vec<Helgamal>,
}

#[derive(Serialize, Deserialize)]
pub struct Witness {
    pub x_vec: Vec<FE>,
    pub r_vec: Vec<FE>,
}

#[derive(Serialize, Deserialize)]
pub struct Proof {
    pub bulletproof: RangeProof,
    pub elgamal_enc: Vec<HomoELGamalProof>,
    pub elgamal_enc_dlog: HomoELGamalDlogProof,
}

impl Proof {
    pub fn prove(
        w: &Witness,
        c: &Helgamalsegmented,
        G: &GE,
        Y: &GE,
        segment_size: &usize,
    ) -> Proof {
        // bulletproofs:
        let num_segments = w.x_vec.len();
        // bit range
        let n = segment_size.clone();
        // batch size
        let m = num_segments;
        let nm = n * m;
        // some seed for generating g and h vectors
        let KZen: &[u8] = &[75, 90, 101, 110];
        let kzen_label = BigInt::from(KZen);

        let g_vec = (0..nm)
            .map(|i| {
                let kzen_label_i = BigInt::from(i as u32) + &kzen_label;
                let hash_i = HSha512::create_hash(&[&kzen_label_i]);
                generate_random_point(&Converter::to_vec(&hash_i))
            })
            .collect::<Vec<GE>>();

        // can run in parallel to g_vec:
        let h_vec = (0..nm)
            .map(|i| {
                let kzen_label_j = BigInt::from(n as u32) + BigInt::from(i as u32) + &kzen_label;
                let hash_j = HSha512::create_hash(&[&kzen_label_j]);
                generate_random_point(&Converter::to_vec(&hash_j))
            })
            .collect::<Vec<GE>>();

        let range_proof =
            RangeProof::prove(&g_vec, &h_vec, &G, &Y, w.x_vec.clone(), &w.r_vec, n.clone());

        // proofs of correct elgamal:

        let elgamal_proofs = (0..num_segments)
            .map(|i| {
                let w = HomoElGamalWitness {
                    r: w.r_vec[i].clone(),
                    x: w.x_vec[i].clone(),
                };
                let delta = HomoElGamalStatement {
                    G: G.clone(),
                    H: G.clone(),
                    Y: Y.clone(),
                    D: c.DE[i].D.clone(),
                    E: c.DE[i].E.clone(),
                };
                HomoELGamalProof::prove(&w, &delta)
            })
            .collect::<Vec<HomoELGamalProof>>();

        // proof of correct ElGamal DLog
        let D_vec: Vec<GE> = (0..num_segments).map(|i| c.DE[i].D.clone()).collect();
        let E_vec: Vec<GE> = (0..num_segments).map(|i| c.DE[i].E.clone()).collect();
        let sum_D = Msegmentation::assemble_ge(&D_vec, segment_size);
        let sum_E = Msegmentation::assemble_ge(&E_vec, segment_size);
        let sum_r = Msegmentation::assemble_fe(&w.r_vec, segment_size);
        let sum_x = Msegmentation::assemble_fe(&w.x_vec, segment_size);
        let Q = G.clone() * &sum_x;
        let delta = HomoElGamalDlogStatement {
            G: G.clone(),
            Y: Y.clone(),
            Q,
            D: sum_D,
            E: sum_E,
        };
        let w = HomoElGamalDlogWitness { r: sum_r, x: sum_x };
        let elgamal_dlog_proof = HomoELGamalDlogProof::prove(&w, &delta);

        Proof {
            bulletproof: range_proof,
            elgamal_enc: elgamal_proofs,
            elgamal_enc_dlog: elgamal_dlog_proof,
        }
    }

    pub fn verify(
        &self,
        c: &Helgamalsegmented,
        G: &GE,
        Y: &GE,
        Q: &GE,
        segment_size: &usize,
    ) -> Result<(), Errors> {
        // bulletproofs:
        let num_segments = self.elgamal_enc.len();
        // bit range
        let n = segment_size.clone();
        // batch size
        let m = num_segments;
        let nm = n * m;
        // some seed for generating g and h vectors
        let KZen: &[u8] = &[75, 90, 101, 110];
        let kzen_label = BigInt::from(KZen);

        let g_vec = (0..nm)
            .map(|i| {
                let kzen_label_i = BigInt::from(i as u32) + &kzen_label;
                let hash_i = HSha512::create_hash(&[&kzen_label_i]);
                generate_random_point(&Converter::to_vec(&hash_i))
            })
            .collect::<Vec<GE>>();

        // can run in parallel to g_vec:
        let h_vec = (0..nm)
            .map(|i| {
                let kzen_label_j = BigInt::from(n as u32) + BigInt::from(i as u32) + &kzen_label;
                let hash_j = HSha512::create_hash(&[&kzen_label_j]);
                generate_random_point(&Converter::to_vec(&hash_j))
            })
            .collect::<Vec<GE>>();

        let D_vec: Vec<GE> = (0..num_segments).map(|i| c.DE[i].D.clone()).collect();
        let bp_ver = self
            .bulletproof
            .verify(&g_vec, &h_vec, G, Y, &D_vec, segment_size.clone())
            .is_ok();

        let elgamal_proofs_ver = (0..num_segments)
            .map(|i| {
                let delta = HomoElGamalStatement {
                    G: G.clone(),
                    H: G.clone(),
                    Y: Y.clone(),
                    D: c.DE[i].D.clone(),
                    E: c.DE[i].E.clone(),
                };
                self.elgamal_enc[i].verify(&delta).is_ok()
            })
            .collect::<Vec<bool>>();

        let E_vec: Vec<GE> = (0..num_segments).map(|i| c.DE[i].E.clone()).collect();
        let sum_D = Msegmentation::assemble_ge(&D_vec, segment_size);
        let sum_E = Msegmentation::assemble_ge(&E_vec, segment_size);

        let delta = HomoElGamalDlogStatement {
            G: G.clone(),
            Y: Y.clone(),
            Q: Q.clone(),
            D: sum_D,
            E: sum_E,
        };

        let elgamal_dlog_proof_ver = self.elgamal_enc_dlog.verify(&delta).is_ok();
        if bp_ver && elgamal_dlog_proof_ver && elgamal_proofs_ver.iter().all(|&x| x == true) {
            Ok(())
        } else {
            Err(ErrorProving)
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SegmentsProof {
    pub bulletproof: RangeProof,
    pub elgamal_enc_dlog: HomoELGamalDlogProof,
    pub D_vec: Vec<GE>,
    pub sum_E: GE,
}

impl SegmentsProof {

    pub fn prove(
        w: &Witness,
        c: &Helgamalsegmented,
        G: &GE,
        Y: &GE,
        &segment_size: usize,
    ) -> (SegmentsProof, Vec<HomoELGamalProof>) {
        let proof = Proof::prove(w, c, G, Y, segment_size);
        let D_vec = c.DE
            .into_iter()
            .map(|DE| DE.D)
            .collect::<Vec<GE>>();
        let E_vec = c.DE
            .into_iter()
            .map(|DE| DE.E)
            .collect::<Vec<GE>>();
        let sum_E = Msegmentation::assemble_ge(&E_vec, segment_size);

        (
            SegmentsProof {
                bulletproof: proof.bulletproof,
                elgamal_enc_dlog: proof.elgamal_enc_dlog,
                D_vec,
                sum_E,
            },
            proof.elgamal_enc
        )
    }

    pub fn verify_initial(
        &self,
        G: &GE,
        Y: &GE,
        Q: &GE,
        &segment_size: usize,
    ) -> Result<(), Errors> {
        // TODO: handle duplicate code

        // bulletproofs:
        let num_segments = self.D_vec.len();
        // bit range
        let n = segment_size.clone();
        // batch size
        let m = num_segments;
        let nm = n * m;
        // some seed for generating g and h vectors
        let KZen: &[u8] = &[75, 90, 101, 110];
        let kzen_label = BigInt::from(KZen);

        let g_vec = (0..nm)
            .map(|i| {
                let kzen_label_i = BigInt::from(i as u32) + &kzen_label;
                let hash_i = HSha512::create_hash(&[&kzen_label_i]);
                generate_random_point(&Converter::to_vec(&hash_i))
            })
            .collect::<Vec<GE>>();

        // can run in parallel to g_vec:
        let h_vec = (0..nm)
            .map(|i| {
                let kzen_label_j = BigInt::from(n as u32) + BigInt::from(i as u32) + &kzen_label;
                let hash_j = HSha512::create_hash(&[&kzen_label_j]);
                generate_random_point(&Converter::to_vec(&hash_j))
            })
            .collect::<Vec<GE>>();

        let bp_ver = self
            .bulletproof
            .verify(&g_vec, &h_vec, G, Y, &self.D_vec, segment_size.clone())
            .is_ok();

        let sum_D = Msegmentation::assemble_ge(&self.D_vec, segment_size);
        let delta = HomoElGamalDlogStatement {
            G: G.clone(),
            Y: Y.clone(),
            Q: Q.clone(),
            D: sum_D,
            E: sum_E,
        };

        let elgamal_dlog_proof_ver = self.elgamal_enc_dlog.verify(&delta).is_ok();
        if bp_ver && elgamal_dlog_proof_ver {
            Ok(())
        } else {
            Err(ErrorProving)
        }
    }

    pub fn verify_segment(
        &self,
        i: usize,
        E_i: &GE,
        elgamal_proof_of_correct_enc: &HomoELGamalProof,
        G: &GE,
        Y: &GE,
        segment_size: usize,
    ) -> Result<(), Errors> {
        // TODO - is it enough?
        let delta = HomoElGamalStatement {
            G: G.clone(),
            H: G.clone(),
            Y: Y.clone(),
            D: self.D_vec[i].clone(),
            E: E_i.clone(),
        };
        self.elgamal_enc[i].verify(&delta)
    }
}

#[cfg(test)]
mod tests {
    use curv::elliptic::curves::traits::*;
    use curv::{FE, GE};
    use juggling::proof_system::*;
    use juggling::segmentation::Msegmentation;
    use wallet::SecretShare;

    #[test]
    fn test_varifiable_encryption() {
        let segment_size = 8;
        let y: FE = ECScalar::new_random();
        let G: GE = ECPoint::generator();
        let Y = G.clone() * &y;
        let x = SecretShare::generate();
        let Q = G.clone() * &x.secret;
        let (segments, encryptions) =
            Msegmentation::to_encrypted_segments(&x.secret, &segment_size, 32, &Y, &G);
        let secret_new = Msegmentation::assemble_fe(&segments.x_vec, &segment_size);
        let secret_decrypted = Msegmentation::decrypt(&encryptions, &G, &y, &segment_size);

        assert_eq!(x.secret, secret_new);
        assert_eq!(x.secret, secret_decrypted.unwrap());

        let proof = Proof::prove(&segments, &encryptions, &G, &Y, &segment_size);
        let result = proof.verify(&encryptions, &G, &Y, &Q, &segment_size);
        assert!(result.is_ok());
    }

    #[test]
    #[should_panic]
    fn test_varifiable_encryption_bad_Q() {
        let segment_size = 8;
        let y: FE = ECScalar::new_random();
        let G: GE = ECPoint::generator();
        let Y = G.clone() * &y;
        let x = SecretShare::generate();
        let Q = G.clone() * &x.secret + G.clone();
        let (segments, encryptions) =
            Msegmentation::to_encrypted_segments(&x.secret, &segment_size, 32, &Y, &G);
        let secret_new = Msegmentation::assemble_fe(&segments.x_vec, &segment_size);
        let secret_decrypted = Msegmentation::decrypt(&encryptions, &G, &y, &segment_size);
        assert_eq!(x.secret, secret_new);
        assert_eq!(x.secret, secret_decrypted.unwrap());

        let proof = Proof::prove(&segments, &encryptions, &G, &Y, &segment_size);
        let result = proof.verify(&encryptions, &G, &Y, &Q, &segment_size);
        assert!(result.is_ok());
    }

    #[test]
    fn test_varifiable_encryption_segment_by_segment() {
        let segment_size = 8;
        let y: FE = ECScalar::new_random();
        let G: GE = ECPoint::generator();
        let Y = G.clone() * &y;
        let x = SecretShare::generate();
        let Q = G.clone() * &x.secret + G.clone();
        let (segments, encryptions) =
            Msegmentation::to_encrypted_segments(&x.secret, &segment_size, 32, &Y, &G);

        // initial proof
        let (proof, elgamal_proofs_of_correct_enc) = SegmentsProof::prove(&segments, &encryptions, &G, &Y, segment_size);

        // initial verify
        let D_vec = encryptions.DE
            .into_iter()
            .map(|DE| DE.D)
            .collect::<Vec<GE>>();

        let initial_result = proof.verify_initial(&D_vec, &G, &Y, &Q, segment_size);
        assert!(initial_result.is_ok());

        // verify segment by segment
        for i in 0..encryptions.DE.len() {
            let result = proof.verify_segment(i, &encryptions.DE[i].E, &elgamal_proofs_of_correct_enc[i], &G, &Y, &Q, segment_size);
            assert!(result.is_ok());
        }
    }
}
