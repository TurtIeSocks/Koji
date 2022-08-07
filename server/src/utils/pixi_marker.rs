#![allow(non_snake_case)]

use crate::models::{
    other::PixiMarker,
    scanner::{Gym, Pokestop, Spawnpoint},
};

pub fn pixi_spawnpoints(incoming: &Vec<Spawnpoint>) -> Vec<PixiMarker> {
    let mut items = Vec::new();
    for i in incoming {
        let icon_id = if i.despawn_sec.is_some() {
            "spawnpoint_true"
        } else {
            "spawnpoint_false"
        };
        items.push(PixiMarker {
            id: i.id.to_string(),
            iconId: icon_id.to_string(),
            position: (i.lat, i.lon),
        });
    }
    return items;
}

pub fn pixi_pokestops(incoming: &Vec<Pokestop>) -> Vec<PixiMarker> {
    let mut items = Vec::new();
    for i in incoming {
        items.push(PixiMarker {
            id: i.id.to_string(),
            iconId: "pokestop".to_string(),
            position: (i.lat, i.lon),
        });
    }
    return items;
}

pub fn pixi_gyms(incoming: &Vec<Gym>) -> Vec<PixiMarker> {
    let mut items = Vec::new();
    for i in incoming {
        items.push(PixiMarker {
            id: i.id.to_string(),
            iconId: "gym".to_string(),
            position: (i.lat, i.lon),
        });
    }
    return items;
}
