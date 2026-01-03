//! This module proivdes struct for stateless validator test fixture.

use reth_stateless::StatelessInput;
use serde::{Deserialize, Serialize};

/// A stateless validation fixture containing block data and witness information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatelessValidatorFixture {
    /// Name of the blockchain test case (e.g., "`ModExpAttackContract`").
    pub name: String,
    /// The stateless input for the block validation.
    pub stateless_input: StatelessInput,
    /// Whether the stateless block validation is successful.
    pub success: bool,
}
