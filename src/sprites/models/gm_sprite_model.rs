use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GMSpriteModel {
    #[serde(rename = "$GMSprite")]
    pub gmsprite: String,

    #[serde(rename = "%Name")]
    pub name_field: String,

    pub bbox_mode: i32,
    pub bbox_bottom: i32,
    pub bbox_left: i32,
    pub bbox_right: i32,
    pub bbox_top: i32,
    pub collision_kind: i32,
    pub collision_tolerance: i32,
    pub dynamic_texture_page: bool,
    pub edge_filtering: bool,

    #[serde(rename = "For3D")]
    pub for_3d: bool,

    pub frames: Vec<GMSpriteFrame>,
    pub grid_x: i32,
    pub grid_y: i32,
    pub height: i32,

    #[serde(rename = "HTile")]
    pub h_tile: bool,

    pub layers: Vec<GMImageLayer>,
    pub name: String,
    pub nine_slice: Option<serde_json::Value>,
    pub origin: i32,
    pub parent: ResourceReference,
    pub pre_multiply_alpha: bool,
    pub resource_type: String,
    pub resource_version: String,
    pub sequence: GMSequence,
    pub swatch_colours: Option<serde_json::Value>,
    pub swf_precision: f64,
    pub texture_group_id: ResourceReference,
    #[serde(rename = "type")]
    pub sprite_type: i32,

    #[serde(rename = "VTile")]
    pub v_tile: bool,

    pub width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GMSpriteFrame {
    #[serde(rename = "$GMSpriteFrame")]
    pub gmsprite_frame: String,

    #[serde(rename = "%Name")]
    pub name_field: String,

    pub name: String,
    pub resource_type: String,
    pub resource_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GMImageLayer {
    #[serde(rename = "$GMImageLayer")]
    pub gmimage_layer: String,

    #[serde(rename = "%Name")]
    pub name_field: String,

    pub blend_mode: i32,
    pub display_name: String,
    pub is_locked: bool,
    pub name: String,
    pub opacity: f64,
    pub resource_type: String,
    pub resource_version: String,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReference {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GMSequence {
    #[serde(rename = "$GMSequence")]
    pub gmsequence: String,

    #[serde(rename = "%Name")]
    pub name_field: String,

    pub auto_record: bool,
    pub backdrop_height: i32,
    pub backdrop_image_opacity: f64,
    pub backdrop_image_path: String,
    pub backdrop_width: i32,
    pub backdrop_x_offset: f64,
    pub backdrop_y_offset: f64,
    pub events: MessageEventKeyframeStore,
    pub event_stub_script: Option<serde_json::Value>,
    pub event_to_function: serde_json::Value,
    pub length: f64,
    pub lock_origin: bool,
    pub moments: MomentsEventKeyframeStore,
    pub name: String,
    pub playback: i32,
    pub playback_speed: f64,
    pub playback_speed_type: i32,
    pub resource_type: String,
    pub resource_version: String,
    pub seq_height: f64,
    pub seq_width: f64,
    pub show_backdrop: bool,
    pub show_backdrop_image: bool,
    pub time_units: i32,
    pub tracks: Vec<GMSpriteFramesTrack>,
    pub visible_range: Option<serde_json::Value>,
    pub volume: f64,
    pub xorigin: i32,
    pub yorigin: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEventKeyframeStore {
    #[serde(rename = "$KeyframeStore<MessageEventKeyframe>")]
    pub keyframe_store: String,

    pub keyframes: Vec<serde_json::Value>,
    pub resource_type: String,
    pub resource_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MomentsEventKeyframeStore {
    #[serde(rename = "$KeyframeStore<MomentsEventKeyframe>")]
    pub keyframe_store: String,

    pub keyframes: Vec<serde_json::Value>,
    pub resource_type: String,
    pub resource_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteFrameKeyframeStore {
    #[serde(rename = "$KeyframeStore<SpriteFrameKeyframe>")]
    pub keyframe_store: String,

    pub keyframes: Vec<SpriteFrameKeyframe>,
    pub resource_type: String,
    pub resource_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GMSpriteFramesTrack {
    #[serde(rename = "$GMSpriteFramesTrack")]
    pub gmsprite_frames_track: String,

    pub builtin_name: i32,
    pub events: Vec<serde_json::Value>,
    pub inherits_track_colour: bool,
    pub interpolation: i32,
    pub is_creation_track: bool,
    pub keyframes: SpriteFrameKeyframeStore,
    pub modifiers: Vec<serde_json::Value>,
    pub name: String,
    pub resource_type: String,
    pub resource_version: String,
    pub sprite_id: Option<serde_json::Value>,
    pub track_colour: i32,
    pub tracks: Vec<serde_json::Value>,
    pub traits: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteFrameKeyframe {
    #[serde(rename = "$Keyframe<SpriteFrameKeyframe>")]
    pub keyframe_sprite_frame_keyframe: String,

    pub channels: KeyframeChannels,
    pub disabled: bool,
    pub id: String,

    #[serde(rename = "IsCreationKey")]
    pub is_creation_key: bool,

    #[serde(rename = "Key")]
    pub key: f64,

    #[serde(rename = "Length")]
    pub length: f64,

    pub resource_type: String,
    pub resource_version: String,

    #[serde(rename = "Stretch")]
    pub stretch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeChannels {
    #[serde(rename = "0")]
    pub channel_0: SpriteFrameKeyframeChannel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteFrameKeyframeChannel {
    #[serde(rename = "$SpriteFrameKeyframe")]
    pub sprite_frame_keyframe: String,

    pub id: ResourceReference,
    pub resource_type: String,
    pub resource_version: String,
}
