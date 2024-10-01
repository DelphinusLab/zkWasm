use super::ExternalHostCallEntry;
use crate::step::StepInfo;

impl TryFrom<&StepInfo> for ExternalHostCallEntry {
    type Error = ();

    fn try_from(value: &StepInfo) -> Result<Self, Self::Error> {
        match value {
            StepInfo::ExternalHostCall { op, value, sig, .. } => Ok(ExternalHostCallEntry {
                op: *op,
                value: value.unwrap(),
                is_ret: sig.is_ret(),
            }),
            _ => Err(()),
        }
    }
}
