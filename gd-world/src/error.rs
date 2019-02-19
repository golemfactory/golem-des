use std::error::Error;
use std::fmt;

use crate::id::Id;
use crate::task::SubTask;

pub enum SimulationError {
    RequestorNotFound(Id),
    ProviderNotFound(Id),
    TaskNotFound(Id),
    RatingNotFound(Id, Id),
    VerificationForSubtaskNotFound(Id, SubTask),
}

impl fmt::Debug for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let data = match self {
            SimulationError::RequestorNotFound(id) => format!("W:requestor R{} not found", id),
            SimulationError::ProviderNotFound(id) => format!("W:provider P{} not found", id),
            SimulationError::TaskNotFound(id) => format!("R{}:task not found", id),
            SimulationError::RatingNotFound(r_id, p_id) => {
                format!("R{}:rating for P{} not found", r_id, p_id)
            }
            SimulationError::VerificationForSubtaskNotFound(id, subtask) => {
                format!("R{}:verification for {} not found", id, subtask)
            }
        };

        write!(f, "{}", data)
    }
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl Error for SimulationError {
    fn description(&self) -> &str {
        "SimulationError"
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
