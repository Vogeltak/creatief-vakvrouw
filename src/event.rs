use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Event {
    #[serde(rename = "event_who_profile_call_name")]
    pub person: String,
    pub event_type: String,
    #[serde(rename = "event_date")]
    pub date: String,
    #[serde(rename = "event_start_end_time")]
    pub start_to_end: String,
    #[serde(rename = "event_starts_at")]
    pub start: String,
    #[serde(rename = "event_ends_at")]
    pub end: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Week {
    pub start_date: String,
    pub end_date: String,
    #[serde(rename = "scheduled_events")]
    pub schedule: Vec<Rooster>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Rooster {
    #[serde(rename = "layer_name")]
    pub name: String,
    #[serde(rename = "layer_days")]
    pub days: Vec<Day>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Day {
    #[serde(rename = "day_key")]
    pub date: String,
    #[serde(rename = "day_events")]
    pub events: Vec<Event>,
}
