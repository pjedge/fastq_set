use failure::{format_err, Error};
use serde::{Deserialize, Serialize};

use crate::read_pair::{ReadPart, WhichRead};
use crate::read_pair_iter::{InputFastqs, ReadPairIter};

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct IlluminaHeaderInfo {
    pub instrument: String,
    pub run_number: u32,
    pub flowcell: String,
    pub lane: u32,
}

impl Default for IlluminaHeaderInfo {
    fn default() -> IlluminaHeaderInfo {
        IlluminaHeaderInfo {
            instrument: "unknown_instrument".to_string(),
            run_number: 0,
            flowcell: "unknow_flowcell".to_string(),
            lane: 0,
        }
    }
}

impl InputFastqs {
    pub fn get_header_info(&self) -> Result<Option<IlluminaHeaderInfo>, Error> {
        let mut iter = ReadPairIter::from_fastq_files(self)?;

        let read1 = iter
            .next()
            .transpose()?
            .ok_or(format_err!("Empty fastq file: {:?}", self))?;

        let header = read1
            .get(WhichRead::R1, ReadPart::Header)
            .ok_or(format_err!("No Read1 in FASTQ data"))?;

        let header = std::str::from_utf8(header)?;

        let header_parts: Vec<&str> = header.split(':').collect();

        if header_parts.len() < 4 {
            Ok(None)
        } else {
            let instrument = header_parts[0].to_string();
            let run_number: u32 = header_parts[1].parse()?;
            let flowcell = header_parts[2].to_string();
            let lane: u32 = header_parts[3].parse()?;

            let res = IlluminaHeaderInfo {
                instrument,
                run_number,
                flowcell,
                lane,
            };

            Ok(Some(res))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::filenames::bcl2fastq::Bcl2FastqDef;
    use crate::filenames::{FindFastqs, LaneSpec};

    #[test]
    fn test_parse_fastq_info() -> Result<(), Error> {
        let path = "tests/filenames/bcl2fastq";

        let query = Bcl2FastqDef {
            fastq_path: path.to_string(),
            sample_name_spec: "Infected".into(),
            lane_spec: LaneSpec::Any,
        };

        let mut fqs = query.find_fastqs()?;
        fqs.sort();

        let info = fqs[0].get_header_info()?;

        let correct = Some(IlluminaHeaderInfo {
            instrument: "A00419".to_string(),
            run_number: 42,
            flowcell: "H7CL3DRXX".to_string(),
            lane: 1,
        });

        assert_eq!(info, correct);

        Ok(())
    }
}
