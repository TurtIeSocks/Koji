table! {
    device (uuid) {
        uuid -> Varchar,
        instance_name -> Nullable<Varchar>,
        last_host -> Nullable<Varchar>,
        last_seen -> Unsigned<Integer>,
        account_username -> Nullable<Varchar>,
        last_lat -> Nullable<Double>,
        last_lon -> Nullable<Double>,
    }
}

table! {
    gym (id) {
        id -> Varchar,
        lat -> Double,
        lon -> Double,
        name -> Nullable<Varchar>,
        url -> Nullable<Varchar>,
        last_modified_timestamp -> Nullable<Unsigned<Integer>>,
        // raid_end_timestamp -> Nullable<Unsigned<Integer>>,
        // raid_spawn_timestamp -> Nullable<Unsigned<Integer>>,
        // raid_battle_timestamp -> Nullable<Unsigned<Integer>>,
        updated -> Unsigned<Integer>,
        // raid_pokemon_id -> Nullable<Unsigned<Smallint>>,
        // guarding_pokemon_id -> Nullable<Unsigned<Smallint>>,
        // available_slots -> Nullable<Unsigned<Smallint>>,
        // availble_slots -> Nullable<Unsigned<Smallint>>,
        // team_id -> Nullable<Unsigned<Tinyint>>,
        // raid_level -> Nullable<Unsigned<Tinyint>>,
        enabled -> Nullable<Unsigned<Tinyint>>,
        // ex_raid_eligible -> Nullable<Unsigned<Tinyint>>,
        // in_battle -> Nullable<Unsigned<Tinyint>>,
        // raid_pokemon_move_1 -> Nullable<Unsigned<Smallint>>,
        // raid_pokemon_move_2 -> Nullable<Unsigned<Smallint>>,
        // raid_pokemon_form -> Nullable<Unsigned<Smallint>>,
        // raid_pokemon_cp -> Nullable<Unsigned<Integer>>,
        // raid_is_exclusive -> Nullable<Unsigned<Tinyint>>,
        cell_id -> Nullable<Unsigned<Bigint>>,
        deleted -> Unsigned<Tinyint>,
        // total_cp -> Nullable<Unsigned<Integer>>,
        first_seen_timestamp -> Unsigned<Integer>,
        // raid_pokemon_gender -> Nullable<Unsigned<Tinyint>>,
        sponsor_id -> Nullable<Unsigned<Smallint>>,
        partner_id -> Nullable<Varchar>,
        // raid_pokemon_costume -> Nullable<Unsigned<Smallint>>,
        // raid_pokemon_evolution -> Nullable<Unsigned<Tinyint>>,
        ar_scan_eligible -> Nullable<Unsigned<Tinyint>>,
        // power_up_level -> Nullable<Unsigned<Smallint>>,
        // power_up_points -> Nullable<Unsigned<Integer>>,
        // power_up_end_timestamp -> Nullable<Unsigned<Integer>>,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::db::sql_types::*;

    instance (name) {
        name -> Varchar,
        #[sql_name = "type"]
        type_ -> Enum,
        data -> Longtext,
    }
}

table! {
    pokestop (id) {
        id -> Varchar,
        lat -> Double,
        lon -> Double,
        name -> Nullable<Varchar>,
        url -> Nullable<Varchar>,
        // lure_expire_timestamp -> Nullable<Unsigned<Integer>>,
        last_modified_timestamp -> Nullable<Unsigned<Integer>>,
        updated -> Unsigned<Integer>,
        enabled -> Nullable<Unsigned<Tinyint>>,
        // quest_type -> Nullable<Unsigned<Integer>>,
        // quest_timestamp -> Nullable<Unsigned<Integer>>,
        // quest_target -> Nullable<Unsigned<Smallint>>,
        // quest_conditions -> Nullable<Text>,
        // quest_rewards -> Nullable<Text>,
        // quest_template -> Nullable<Varchar>,
        // quest_title -> Nullable<Varchar>,
        // quest_reward_type -> Nullable<Unsigned<Smallint>>,
        // quest_item_id -> Nullable<Unsigned<Smallint>>,
        // quest_reward_amount -> Nullable<Unsigned<Smallint>>,
        cell_id -> Nullable<Unsigned<Bigint>>,
        deleted -> Unsigned<Tinyint>,
        // lure_id -> Nullable<Smallint>,
        first_seen_timestamp -> Unsigned<Integer>,
        sponsor_id -> Nullable<Unsigned<Smallint>>,
        partner_id -> Nullable<Varchar>,
        // quest_pokemon_id -> Nullable<Unsigned<Smallint>>,
        ar_scan_eligible -> Nullable<Unsigned<Tinyint>>,
        // power_up_level -> Nullable<Unsigned<Smallint>>,
        // power_up_points -> Nullable<Unsigned<Integer>>,
        // power_up_end_timestamp -> Nullable<Unsigned<Integer>>,
        // alternative_quest_type -> Nullable<Unsigned<Integer>>,
        // alternative_quest_timestamp -> Nullable<Unsigned<Integer>>,
        // alternative_quest_target -> Nullable<Unsigned<Smallint>>,
        // alternative_quest_conditions -> Nullable<Text>,
        // alternative_quest_rewards -> Nullable<Text>,
        // alternative_quest_template -> Nullable<Varchar>,
        // alternative_quest_title -> Nullable<Varchar>,
        // alternative_quest_pokemon_id -> Nullable<Unsigned<Smallint>>,
        // alternative_quest_reward_type -> Nullable<Unsigned<Smallint>>,
        // alternative_quest_item_id -> Nullable<Unsigned<Smallint>>,
        // alternative_quest_reward_amount -> Nullable<Unsigned<Smallint>>,
    }
}

table! {
    spawnpoint (id) {
        id -> Unsigned<Bigint>,
        lat -> Double,
        lon -> Double,
        updated -> Unsigned<Integer>,
        last_seen -> Unsigned<Integer>,
        despawn_sec -> Nullable<Unsigned<Smallint>>,
    }
}

joinable!(device -> instance (instance_name));

allow_tables_to_appear_in_same_query!(device, gym, instance, pokestop, spawnpoint,);
