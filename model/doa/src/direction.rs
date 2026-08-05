use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use std::str::FromStr;

/// Compass direction derived from the DoA angle.
/// Default quadrant mapping (after calibration):
/// - 315°–45° → North (front)
/// - 45°–135° → East (right)
/// - 135°–225° → South (back)
/// - 225°–315° → West (left)
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DoaDirection {
    /// North side of the table (315°–45°). Default front of the mic array.
    #[default]
    North,
    /// East side of the table (45°–135°).
    East,
    /// South side of the table (135°–225°).
    South,
    /// West side of the table (225°–315°).
    West,
}

impl DoaDirection {
    /// Maps a calibrated DoA angle (0-359) to a compass direction.
    /// Uses the default 45°-offset quadrant mapping.
    pub fn from_angle(angle: u16) -> Self {
        let angle = angle % 360;
        if angle >= 315 || angle < 45 {
            Self::North
        } else if angle < 135 {
            Self::East
        } else if angle < 225 {
            Self::South
        } else {
            Self::West
        }
    }

    /// Maps a raw DoA angle to a compass direction, applying a rotation offset.
    /// The offset compensates for the physical mounting orientation of the
    /// microphone array relative to the table's reference direction (North).
    /// Positive offsets rotate clockwise, negative offsets counter-clockwise.
    /// For example, if the DSP's 0° axis points 90° clockwise from the table's
    /// North, set `offset = -90` (or equivalently `offset = 270`).
    pub fn from_angle_with_offset(raw_angle: u16, offset: i16) -> Self {
        let calibrated_angle = (raw_angle as i16 + offset).rem_euclid(360) as u16;
        Self::from_angle(calibrated_angle)
    }

    /// Returns a human-readable label key for the direction.
    pub fn label_key(&self) -> &'static str {
        match self {
            Self::North => "doa_direction_north",
            Self::East => "doa_direction_east",
            Self::South => "doa_direction_south",
            Self::West => "doa_direction_west",
        }
    }
}

impl Display for DoaDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::North => f.write_str("North"),
            Self::East => f.write_str("East"),
            Self::South => f.write_str("South"),
            Self::West => f.write_str("West"),
        }
    }
}

impl FromStr for DoaDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "north" => Ok(Self::North),
            "east" => Ok(Self::East),
            "south" => Ok(Self::South),
            "west" => Ok(Self::West),
            _ => Err(format!("Unknown direction: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DoaDirection;
    use std::str::FromStr;

    #[test]
    fn test_from_angle_north() {
        assert_eq!(DoaDirection::from_angle(0), DoaDirection::North);
        assert_eq!(DoaDirection::from_angle(44), DoaDirection::North);
        assert_eq!(DoaDirection::from_angle(315), DoaDirection::North);
        assert_eq!(DoaDirection::from_angle(359), DoaDirection::North);
    }

    #[test]
    fn test_from_angle_east() {
        assert_eq!(DoaDirection::from_angle(45), DoaDirection::East);
        assert_eq!(DoaDirection::from_angle(90), DoaDirection::East);
        assert_eq!(DoaDirection::from_angle(134), DoaDirection::East);
    }

    #[test]
    fn test_from_angle_south() {
        assert_eq!(DoaDirection::from_angle(135), DoaDirection::South);
        assert_eq!(DoaDirection::from_angle(180), DoaDirection::South);
        assert_eq!(DoaDirection::from_angle(224), DoaDirection::South);
    }

    #[test]
    fn test_from_angle_west() {
        assert_eq!(DoaDirection::from_angle(225), DoaDirection::West);
        assert_eq!(DoaDirection::from_angle(270), DoaDirection::West);
        assert_eq!(DoaDirection::from_angle(314), DoaDirection::West);
    }

    #[test]
    fn test_from_angle_wraps_360() {
        assert_eq!(DoaDirection::from_angle(360), DoaDirection::North);
        assert_eq!(DoaDirection::from_angle(405), DoaDirection::East);
        assert_eq!(DoaDirection::from_angle(720), DoaDirection::North);
    }

    #[test]
    fn test_from_angle_with_offset_zero() {
        assert_eq!(DoaDirection::from_angle_with_offset(90, 0), DoaDirection::East);
    }

    #[test]
    fn test_from_angle_with_offset_positive() {
        assert_eq!(DoaDirection::from_angle_with_offset(0, 90), DoaDirection::East);
        assert_eq!(DoaDirection::from_angle_with_offset(0, 180), DoaDirection::South);
        assert_eq!(DoaDirection::from_angle_with_offset(0, 270), DoaDirection::West);
    }

    #[test]
    fn test_from_angle_with_offset_negative() {
        assert_eq!(DoaDirection::from_angle_with_offset(90, -90), DoaDirection::North);
        assert_eq!(DoaDirection::from_angle_with_offset(0, -90), DoaDirection::West);
        assert_eq!(DoaDirection::from_angle_with_offset(180, -270), DoaDirection::West);
    }

    #[test]
    fn test_from_angle_with_offset_wraps() {
        assert_eq!(DoaDirection::from_angle_with_offset(0, 360), DoaDirection::North);
        assert_eq!(DoaDirection::from_angle_with_offset(0, -360), DoaDirection::North);
        assert_eq!(DoaDirection::from_angle_with_offset(180, 360), DoaDirection::South);
    }

    #[test]
    fn test_label_key() {
        assert_eq!(DoaDirection::North.label_key(), "doa_direction_north");
        assert_eq!(DoaDirection::East.label_key(), "doa_direction_east");
        assert_eq!(DoaDirection::South.label_key(), "doa_direction_south");
        assert_eq!(DoaDirection::West.label_key(), "doa_direction_west");
    }

    #[test]
    fn test_display() {
        assert_eq!(DoaDirection::North.to_string(), "North");
        assert_eq!(DoaDirection::East.to_string(), "East");
        assert_eq!(DoaDirection::South.to_string(), "South");
        assert_eq!(DoaDirection::West.to_string(), "West");
    }

    #[test]
    fn test_from_str_valid() {
        assert_eq!(DoaDirection::from_str("north").unwrap(), DoaDirection::North);
        assert_eq!(DoaDirection::from_str("NORTH").unwrap(), DoaDirection::North);
        assert_eq!(DoaDirection::from_str("East").unwrap(), DoaDirection::East);
        assert_eq!(DoaDirection::from_str("south").unwrap(), DoaDirection::South);
        assert_eq!(DoaDirection::from_str("west").unwrap(), DoaDirection::West);
    }

    #[test]
    fn test_from_str_invalid() {
        assert!(DoaDirection::from_str("northeast").is_err());
        assert!(DoaDirection::from_str("").is_err());
        assert!(DoaDirection::from_str("foo").is_err());
    }

    #[test]
    fn test_serde_round_trip() {
        let json = serde_json::to_string(&DoaDirection::South).unwrap();
        assert_eq!(json, "\"South\"");
        let deserialized: DoaDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, DoaDirection::South);
    }

    #[test]
    fn test_default() {
        assert_eq!(DoaDirection::default(), DoaDirection::North);
    }
}
