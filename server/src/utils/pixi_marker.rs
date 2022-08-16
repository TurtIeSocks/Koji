#![allow(non_snake_case)]

use crate::models::{other::PixiMarker, scanner::GenericData};

pub fn pixi_marker(incoming: &Vec<GenericData>, category: &str) -> Vec<PixiMarker> {
    let mut items = Vec::new();
    for i in incoming {
        let icon_id = match category {
            "gym" => "g",
            "pokestop" => "p",
            "spawnpoint" => match i.verified {
                Some(_) => "v",
                _ => "u",
            },
            _ => "o",
        };
        items.push(PixiMarker {
            id: i.id.to_string(),
            iconId: icon_id.to_string(),
            position: (i.lat, i.lon),
        });
    }
    return items;
}
