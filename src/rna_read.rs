// Copyright (c) 2018 10x Genomics, Inc. All rights reserved.

//! ReadPair wrapper object for RNA reads from Single Cell 3' nad Single Cell 5' / VDJ ibraries. 
//! Provides access to the barcode and allows for dynamic trimming.

use std::collections::HashMap;
use read_pair::{ReadPart, ReadPair, WhichRead, RpRange};
use {Barcode, HasBarcode, Umi, FastqProcessor, InputFastqs};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ChemistryDef {
    barcode_read_length: usize,
    barcode_read_offset: usize,
    barcode_read_type: WhichRead,
    barcode_whitelist: String,
    description: String,
    endedness: Option<String>,
    name: String,
    read_type_to_bcl2fastq_filename: HashMap<String, Option<String>>,
    read_type_to_bcl_processor_filename: HashMap<String, Option<String>>,
    rna_read2_length: Option<usize>,
    rna_read2_offset: usize,
    rna_read2_type: Option<WhichRead>,
    rna_read_length: Option<usize>,
    rna_read_offset: usize,
    rna_read_type: WhichRead,
    si_read_length: Option<usize>,
    si_read_offset: usize,
    si_read_type: WhichRead,
    strandedness: String,
    umi_read_length: usize,
    umi_read_offset: usize,
    umi_read_type: WhichRead
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct RnaChunk {
    chemistry: ChemistryDef,
    gem_group: u16,
    read_chunks: HashMap<WhichRead, Option<String>>,
    read_group: String,
    reads_interleaved: bool,
    chunk_id: Option<u16>,
}


impl FastqProcessor<RnaRead> for RnaChunk {
    fn process_read(&self, read: ReadPair) -> Option<RnaRead> {
        let chem = &self.chemistry;
        let bc_range = RpRange::new(chem.barcode_read_type, chem.barcode_read_offset, Some(chem.barcode_read_length));
        let umi_range = RpRange::new(chem.umi_read_type, chem.umi_read_offset, Some(chem.umi_read_length));
        let r1 = RpRange::new(chem.rna_read_type, chem.rna_read_offset, chem.rna_read_length);

        let r2 = match chem.rna_read2_type {
            Some(read) => Some(RpRange::new(read, chem.rna_read2_offset, chem.rna_read2_length)),
            None => None,
        };

        let barcode = Barcode::new(self.gem_group, read.get_range(&bc_range, ReadPart::Seq).unwrap(), true);
        let umi = Umi::new(read.get_range(&umi_range, ReadPart::Seq).unwrap());

        Some(
            RnaRead {
                data: read,
                bc: barcode,
                umi: umi,
                bc_range: bc_range,
                umi_range: umi_range,
                r1_range: r1,
                r2_range: r2,
                chunk_id: self.chunk_id.unwrap(),
        })
    }

    fn fastq_files(&self) -> InputFastqs {
        let r1 = self.read_chunks.get(&WhichRead::R1).unwrap().clone().unwrap().clone();
        InputFastqs {
            r1: r1,
            r2: self.read_chunks.get(&WhichRead::R2).unwrap_or(&None).clone(),
            i1: self.read_chunks.get(&WhichRead::I1).unwrap_or(&None).clone(),
            i2: self.read_chunks.get(&WhichRead::I2).unwrap_or(&None).clone(),
            r1_interleaved: self.reads_interleaved,
        }
    }
    fn description(&self) -> String { "blah".to_string() }
    fn bc_subsample_rate(&self) -> f64 { 0.0 }
    fn read_subsample_rate(&self) -> f64 { 0.0 }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RnaRead {
    data: ReadPair,
    bc: Barcode,
    umi: Umi,
    bc_range: RpRange,
    umi_range: RpRange,
    r1_range: RpRange,
    r2_range: Option<RpRange>,
    chunk_id: u16,
}

impl HasBarcode for RnaRead {
    fn barcode(&self) -> Barcode {
        self.bc
    }

    fn set_barcode(&mut self, barcode: Barcode) {
        self.bc = barcode;
    }

    fn barcode_qual(&self) -> &[u8] {
        self.raw_bc_qual()
    }
}

impl RnaRead {


        /// FASTQ read header
    pub fn header(&self) -> &[u8] {
        self.data.get(WhichRead::R1, ReadPart::Header).unwrap()
    }

    /// Full raw R1 sequence
    pub fn r1_seq_raw(&self) -> &[u8] {
        self.data.get(WhichRead::R1, ReadPart::Seq).unwrap()
    }

    /// Full raw R1 QVs
    pub fn r1_qual_raw(&self) -> &[u8] {
        self.data.get(WhichRead::R1, ReadPart::Qual).unwrap()
    }

    /// Full R2 sequence
    pub fn r2_seq_raw(&self) -> &[u8] {
        self.data.get(WhichRead::R2, ReadPart::Seq).unwrap()
    }

    /// Full R2 QVs
    pub fn r2_qual_raw(&self) -> &[u8] {
        self.data.get(WhichRead::R2, ReadPart::Qual).unwrap()
    }

    /// Sample index (I1) sequence
    pub fn si_seq(&self) -> Option<&[u8]> {
        self.data.get(WhichRead::I1, ReadPart::Seq)
    }

    /// Sample index (I1) QVs
    pub fn si_qual(&self) -> Option<&[u8]> {
        self.data.get(WhichRead::I1, ReadPart::Qual)
    }

    /// Raw, uncorrected barcode sequence
    pub fn raw_bc_seq(&self) -> &[u8] {
        self.data.get_range(&self.bc_range, ReadPart::Seq).unwrap()
    }

    /// Raw barcode QVs
    pub fn raw_bc_qual(&self) -> &[u8] {
        self.data.get_range(&self.bc_range, ReadPart::Qual).unwrap()
    }

    /// Raw, uncorrected barcode sequence
    pub fn raw_umi_seq(&self) -> &[u8] {
        self.data.get_range(&self.umi_range, ReadPart::Seq).unwrap()
    }

    /// Raw barcode QVs
    pub fn raw_umi_qual(&self) -> &[u8] {
        self.data.get_range(&self.umi_range, ReadPart::Qual).unwrap()
    }

    /// Usable R1 bases after removal of BC and trimming
    pub fn r1_seq(&self) -> &[u8] {
        self.data.get_range(&self.r1_range, ReadPart::Seq).unwrap()
    }

    /// Usable R1 bases after removal of BC and trimming
    pub fn r1_qual(&self) -> &[u8] {
        self.data.get_range(&self.r1_range, ReadPart::Qual).unwrap()
    }
}



#[cfg(test)]
mod test_rna_cfg {
    use super::*;
    use serde_json;

    #[test]
    fn test_vdj_cfg() {
        let c: Vec<RnaChunk> = serde_json::from_str(VDJ_CFG).unwrap();
        println!("{:?}", c);
    }

    #[test]
    fn test_gex_cfg() {
        let c: Vec<RnaChunk> = serde_json::from_str(GEX_CFG).unwrap();
        println!("{:?}", c);
    }

    const GEX_CFG : &str = r#"
    [
        {
            "chemistry": {
                "barcode_read_length": 16,
                "barcode_read_offset": 0,
                "barcode_read_type": "R1",
                "barcode_whitelist": "3M-february-2018",
                "description": "custom",
                "name": "custom",
                "read_type_to_bcl2fastq_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "R1",
                    "R2": "R2"
                },
                "read_type_to_bcl_processor_filename": {
                    "I1": "I1",
                    "I2": "I2",
                    "R1": "RA",
                    "R2": null
                },
                "rna_read2_length": null,
                "rna_read2_offset": 0,
                "rna_read2_type": null,
                "rna_read_length": null,
                "rna_read_offset": 0,
                "rna_read_type": "R2",
                "si_read_length": null,
                "si_read_offset": 0,
                "si_read_type": "I1",
                "strandedness": "+",
                "umi_read_length": 10,
                "umi_read_offset": 16,
                "umi_read_type": "R1"
            },
            "gem_group": 1,
            "library_type": "Gene Expression",
            "read_chunks": {
                "I1": "/mnt/analysis/marsoc/pipestances/HCWYJDMXX/BCL_PROCESSOR_PD/HCWYJDMXX/1015.12.14-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u0194e1d3c4/files/demultiplexed_fastq_path/read-I1_si-GGTATGCA_lane-001-chunk-001.fastq.gz",
                "I2": null,
                "R1": "/mnt/analysis/marsoc/pipestances/HCWYJDMXX/BCL_PROCESSOR_PD/HCWYJDMXX/1015.12.14-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u0194e1d3c4/files/demultiplexed_fastq_path/read-RA_si-GGTATGCA_lane-001-chunk-001.fastq.gz",
                "R2": null
            },
            "read_group": "67693:MissingLibrary:1:HCWYJDMXX:1",
            "reads_interleaved": true
        },
        {
            "chemistry": {
                "barcode_read_length": 16,
                "barcode_read_offset": 0,
                "barcode_read_type": "R1",
                "barcode_whitelist": "3M-february-2018",
                "description": "custom",
                "name": "custom",
                "read_type_to_bcl2fastq_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "R1",
                    "R2": "R2"
                },
                "read_type_to_bcl_processor_filename": {
                    "I1": "I1",
                    "I2": "I2",
                    "R1": "RA",
                    "R2": null
                },
                "rna_read2_length": null,
                "rna_read2_offset": 0,
                "rna_read2_type": null,
                "rna_read_length": null,
                "rna_read_offset": 0,
                "rna_read_type": "R2",
                "si_read_length": null,
                "si_read_offset": 0,
                "si_read_type": "I1",
                "strandedness": "+",
                "umi_read_length": 10,
                "umi_read_offset": 16,
                "umi_read_type": "R1"
            },
            "gem_group": 1,
            "library_type": "Gene Expression",
            "read_chunks": {
                "I1": "/mnt/analysis/marsoc/pipestances/HCWYJDMXX/BCL_PROCESSOR_PD/HCWYJDMXX/1015.12.14-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u0194e1d3c4/files/demultiplexed_fastq_path/read-I1_si-CTCGAAAT_lane-001-chunk-001.fastq.gz",
                "I2": null,
                "R1": "/mnt/analysis/marsoc/pipestances/HCWYJDMXX/BCL_PROCESSOR_PD/HCWYJDMXX/1015.12.14-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u0194e1d3c4/files/demultiplexed_fastq_path/read-RA_si-CTCGAAAT_lane-001-chunk-001.fastq.gz",
                "R2": null
            },
            "read_group": "67693:MissingLibrary:1:HCWYJDMXX:1",
            "reads_interleaved": true
        },
        {
            "chemistry": {
                "barcode_read_length": 16,
                "barcode_read_offset": 0,
                "barcode_read_type": "R1",
                "barcode_whitelist": "3M-february-2018",
                "description": "custom",
                "name": "custom",
                "read_type_to_bcl2fastq_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "R1",
                    "R2": "R2"
                },
                "read_type_to_bcl_processor_filename": {
                    "I1": "I1",
                    "I2": "I2",
                    "R1": "RA",
                    "R2": null
                },
                "rna_read2_length": null,
                "rna_read2_offset": 0,
                "rna_read2_type": null,
                "rna_read_length": null,
                "rna_read_offset": 0,
                "rna_read_type": "R2",
                "si_read_length": null,
                "si_read_offset": 0,
                "si_read_type": "I1",
                "strandedness": "+",
                "umi_read_length": 10,
                "umi_read_offset": 16,
                "umi_read_type": "R1"
            },
            "gem_group": 1,
            "library_type": "Gene Expression",
            "read_chunks": {
                "I1": "/mnt/analysis/marsoc/pipestances/HCWYJDMXX/BCL_PROCESSOR_PD/HCWYJDMXX/1015.12.14-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u0194e1d3c4/files/demultiplexed_fastq_path/read-I1_si-ACACCTTC_lane-001-chunk-001.fastq.gz",
                "I2": null,
                "R1": "/mnt/analysis/marsoc/pipestances/HCWYJDMXX/BCL_PROCESSOR_PD/HCWYJDMXX/1015.12.14-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u0194e1d3c4/files/demultiplexed_fastq_path/read-RA_si-ACACCTTC_lane-001-chunk-001.fastq.gz",
                "R2": null
            },
            "read_group": "67693:MissingLibrary:1:HCWYJDMXX:1",
            "reads_interleaved": true
        },
        {
            "chemistry": {
                "barcode_read_length": 16,
                "barcode_read_offset": 0,
                "barcode_read_type": "R1",
                "barcode_whitelist": "3M-february-2018",
                "description": "custom",
                "name": "custom",
                "read_type_to_bcl2fastq_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "R1",
                    "R2": "R2"
                },
                "read_type_to_bcl_processor_filename": {
                    "I1": "I1",
                    "I2": "I2",
                    "R1": "RA",
                    "R2": null
                },
                "rna_read2_length": null,
                "rna_read2_offset": 0,
                "rna_read2_type": null,
                "rna_read_length": null,
                "rna_read_offset": 0,
                "rna_read_type": "R2",
                "si_read_length": null,
                "si_read_offset": 0,
                "si_read_type": "I1",
                "strandedness": "+",
                "umi_read_length": 10,
                "umi_read_offset": 16,
                "umi_read_type": "R1"
            },
            "gem_group": 1,
            "library_type": "Gene Expression",
            "read_chunks": {
                "I1": "/mnt/analysis/marsoc/pipestances/HCWYJDMXX/BCL_PROCESSOR_PD/HCWYJDMXX/1015.12.14-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u0194e1d3c4/files/demultiplexed_fastq_path/read-I1_si-TAGTGCGG_lane-001-chunk-001.fastq.gz",
                "I2": null,
                "R1": "/mnt/analysis/marsoc/pipestances/HCWYJDMXX/BCL_PROCESSOR_PD/HCWYJDMXX/1015.12.14-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u0194e1d3c4/files/demultiplexed_fastq_path/read-RA_si-TAGTGCGG_lane-001-chunk-001.fastq.gz",
                "R2": null
            },
            "read_group": "67693:MissingLibrary:1:HCWYJDMXX:1",
            "reads_interleaved": true
        }
    ]"#;

    const VDJ_CFG : &str = r#"
    [
        {
            "chemistry": {
                "barcode_read_length": 16,
                "barcode_read_offset": 0,
                "barcode_read_type": "R1",
                "barcode_whitelist": "737K-august-2016",
                "description": "Single Cell V(D)J",
                "endedness": "five_prime",
                "name": "SCVDJ",
                "read_type_to_bcl2fastq_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "R1",
                    "R2": "R2"
                },
                "read_type_to_bcl_processor_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "RA",
                    "R2": null
                },
                "rna_read2_length": null,
                "rna_read2_offset": 0,
                "rna_read2_type": "R2",
                "rna_read_length": null,
                "rna_read_offset": 41,
                "rna_read_type": "R1",
                "si_read_length": null,
                "si_read_offset": 0,
                "si_read_type": "I1",
                "strandedness": "+",
                "umi_read_length": 10,
                "umi_read_offset": 16,
                "umi_read_type": "R1"
            },
            "gem_group": 1,
            "library_type": "Gene Expression",
            "read_chunks": {
                "I1": "/mnt/analysis/marsoc/pipestances/D3VJ6/BCL_PROCESSOR_PD/D3VJ6/1015.12.13-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u30b0b3b34c/files/demultiplexed_fastq_path/read-I1_si-GGTTTACT_lane-001-chunk-000.fastq.gz",
                "I2": null,
                "R1": "/mnt/analysis/marsoc/pipestances/D3VJ6/BCL_PROCESSOR_PD/D3VJ6/1015.12.13-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u30b0b3b34c/files/demultiplexed_fastq_path/read-RA_si-GGTTTACT_lane-001-chunk-000.fastq.gz",
                "R2": null
            },
            "read_group": "62336:MissingLibrary:1:000000000-D3VJ6:1",
            "reads_interleaved": true
        },
        {
            "chemistry": {
                "barcode_read_length": 16,
                "barcode_read_offset": 0,
                "barcode_read_type": "R1",
                "barcode_whitelist": "737K-august-2016",
                "description": "Single Cell V(D)J",
                "endedness": "five_prime",
                "name": "SCVDJ",
                "read_type_to_bcl2fastq_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "R1",
                    "R2": "R2"
                },
                "read_type_to_bcl_processor_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "RA",
                    "R2": null
                },
                "rna_read2_length": null,
                "rna_read2_offset": 0,
                "rna_read2_type": "R2",
                "rna_read_length": null,
                "rna_read_offset": 41,
                "rna_read_type": "R1",
                "si_read_length": null,
                "si_read_offset": 0,
                "si_read_type": "I1",
                "strandedness": "+",
                "umi_read_length": 10,
                "umi_read_offset": 16,
                "umi_read_type": "R1"
            },
            "gem_group": 1,
            "library_type": "Gene Expression",
            "read_chunks": {
                "I1": "/mnt/analysis/marsoc/pipestances/D3VJ6/BCL_PROCESSOR_PD/D3VJ6/1015.12.13-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u30b0b3b34c/files/demultiplexed_fastq_path/read-I1_si-CTAAACGG_lane-001-chunk-000.fastq.gz",
                "I2": null,
                "R1": "/mnt/analysis/marsoc/pipestances/D3VJ6/BCL_PROCESSOR_PD/D3VJ6/1015.12.13-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u30b0b3b34c/files/demultiplexed_fastq_path/read-RA_si-CTAAACGG_lane-001-chunk-000.fastq.gz",
                "R2": null
            },
            "read_group": "62336:MissingLibrary:1:000000000-D3VJ6:1",
            "reads_interleaved": true
        },
        {
            "chemistry": {
                "barcode_read_length": 16,
                "barcode_read_offset": 0,
                "barcode_read_type": "R1",
                "barcode_whitelist": "737K-august-2016",
                "description": "Single Cell V(D)J",
                "endedness": "five_prime",
                "name": "SCVDJ",
                "read_type_to_bcl2fastq_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "R1",
                    "R2": "R2"
                },
                "read_type_to_bcl_processor_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "RA",
                    "R2": null
                },
                "rna_read2_length": null,
                "rna_read2_offset": 0,
                "rna_read2_type": "R2",
                "rna_read_length": null,
                "rna_read_offset": 41,
                "rna_read_type": "R1",
                "si_read_length": null,
                "si_read_offset": 0,
                "si_read_type": "I1",
                "strandedness": "+",
                "umi_read_length": 10,
                "umi_read_offset": 16,
                "umi_read_type": "R1"
            },
            "gem_group": 1,
            "library_type": "Gene Expression",
            "read_chunks": {
                "I1": "/mnt/analysis/marsoc/pipestances/D3VJ6/BCL_PROCESSOR_PD/D3VJ6/1015.12.13-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u30b0b3b34c/files/demultiplexed_fastq_path/read-I1_si-TCGGCGTC_lane-001-chunk-000.fastq.gz",
                "I2": null,
                "R1": "/mnt/analysis/marsoc/pipestances/D3VJ6/BCL_PROCESSOR_PD/D3VJ6/1015.12.13-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u30b0b3b34c/files/demultiplexed_fastq_path/read-RA_si-TCGGCGTC_lane-001-chunk-000.fastq.gz",
                "R2": null
            },
            "read_group": "62336:MissingLibrary:1:000000000-D3VJ6:1",
            "reads_interleaved": true
        },
        {
            "chemistry": {
                "barcode_read_length": 16,
                "barcode_read_offset": 0,
                "barcode_read_type": "R1",
                "barcode_whitelist": "737K-august-2016",
                "description": "Single Cell V(D)J",
                "endedness": "five_prime",
                "name": "SCVDJ",
                "read_type_to_bcl2fastq_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "R1",
                    "R2": "R2"
                },
                "read_type_to_bcl_processor_filename": {
                    "I1": "I1",
                    "I2": null,
                    "R1": "RA",
                    "R2": null
                },
                "rna_read2_length": null,
                "rna_read2_offset": 0,
                "rna_read2_type": "R2",
                "rna_read_length": null,
                "rna_read_offset": 41,
                "rna_read_type": "R1",
                "si_read_length": null,
                "si_read_offset": 0,
                "si_read_type": "I1",
                "strandedness": "+",
                "umi_read_length": 10,
                "umi_read_offset": 16,
                "umi_read_type": "R1"
            },
            "gem_group": 1,
            "library_type": "Gene Expression",
            "read_chunks": {
                "I1": "/mnt/analysis/marsoc/pipestances/D3VJ6/BCL_PROCESSOR_PD/D3VJ6/1015.12.13-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u30b0b3b34c/files/demultiplexed_fastq_path/read-I1_si-AACCGTAA_lane-001-chunk-000.fastq.gz",
                "I2": null,
                "R1": "/mnt/analysis/marsoc/pipestances/D3VJ6/BCL_PROCESSOR_PD/D3VJ6/1015.12.13-0/BCL_PROCESSOR_PD/BCL_PROCESSOR/MERGE_FASTQS_FROM_TILES/fork0/join-u30b0b3b34c/files/demultiplexed_fastq_path/read-RA_si-AACCGTAA_lane-001-chunk-000.fastq.gz",
                "R2": null
            },
            "read_group": "62336:MissingLibrary:1:000000000-D3VJ6:1",
            "reads_interleaved": true
        }
    ]
    "#;
}