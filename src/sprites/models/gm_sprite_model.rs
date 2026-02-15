use serde::{Deserialize, Serialize};

use crate::sprites::bbox::BBox;

impl GMSpriteModel {
    /// Build a complete `GMSpriteModel` ready to be serialized as a `.yy` file.
    ///
    /// * `name`        - sprite resource name (e.g. "sPlayerIdle")
    /// * `width`/`height` - dimensions of every frame in pixels
    /// * `frame_guids` - one UUID string per frame, already generated
    /// * `layer_guid`  - the single image-layer UUID shared by all frames
    /// * `parent`      - the GM folder reference (name + folderPath)
    /// * `bbox`        - tight bounding box computed from pixel data, or None if fully transparent
    pub fn new(
        name: &str,
        width: i32,
        height: i32,
        frame_guids: &[String],
        layer_guid: &str,
        parent: ResourceReference,
        bbox: Option<BBox>,
    ) -> Self {
        let bbox = bbox.unwrap_or(BBox {
            left: 0,
            top: 0,
            right: width - 1,
            bottom: height - 1,
        });

        let sprite_yy_path = format!("sprites/{name}/{name}.yy");

        let frames: Vec<GMSpriteFrame> = frame_guids
            .iter()
            .map(|guid| GMSpriteFrame {
                gmsprite_frame: "v1".to_string(),
                name_field: guid.clone(),
                name: guid.clone(),
                resource_type: "GMSpriteFrame".to_string(),
                resource_version: "2.0".to_string(),
            })
            .collect();

        let keyframes: Vec<SpriteFrameKeyframe> = frame_guids
            .iter()
            .enumerate()
            .map(|(i, guid)| SpriteFrameKeyframe {
                keyframe_sprite_frame_keyframe: String::new(),
                channels: KeyframeChannels {
                    channel_0: SpriteFrameKeyframeChannel {
                        sprite_frame_keyframe: String::new(),
                        id: ResourceReference {
                            name: guid.clone(),
                            path: sprite_yy_path.clone(),
                        },
                        resource_type: "SpriteFrameKeyframe".to_string(),
                        resource_version: "2.0".to_string(),
                    },
                },
                disabled: false,
                id: uuid::Uuid::new_v4().to_string(),
                is_creation_key: false,
                key: i as f64,
                length: 1.0,
                resource_type: "Keyframe<SpriteFrameKeyframe>".to_string(),
                resource_version: "2.0".to_string(),
                stretch: false,
            })
            .collect();

        let track = GMSpriteFramesTrack {
            gmsprite_frames_track: String::new(),
            builtin_name: 0,
            events: Vec::new(),
            inherits_track_colour: true,
            interpolation: 1,
            is_creation_track: false,
            keyframes: SpriteFrameKeyframeStore {
                keyframe_store: String::new(),
                keyframes,
                resource_type: "KeyframeStore<SpriteFrameKeyframe>".to_string(),
                resource_version: "2.0".to_string(),
            },
            modifiers: Vec::new(),
            name: "frames".to_string(),
            resource_type: "GMSpriteFramesTrack".to_string(),
            resource_version: "2.0".to_string(),
            sprite_id: None,
            track_colour: 0,
            tracks: Vec::new(),
            traits: 0,
        };

        let sequence = GMSequence {
            gmsequence: "v1".to_string(),
            name_field: name.to_string(),
            auto_record: true,
            backdrop_height: 768,
            backdrop_image_opacity: 0.5,
            backdrop_image_path: String::new(),
            backdrop_width: 1366,
            backdrop_x_offset: 0.0,
            backdrop_y_offset: 0.0,
            events: MessageEventKeyframeStore {
                keyframe_store: String::new(),
                keyframes: Vec::new(),
                resource_type: "KeyframeStore<MessageEventKeyframe>".to_string(),
                resource_version: "2.0".to_string(),
            },
            event_stub_script: None,
            event_to_function: serde_json::Value::Object(serde_json::Map::new()),
            length: frame_guids.len() as f64,
            lock_origin: false,
            moments: MomentsEventKeyframeStore {
                keyframe_store: String::new(),
                keyframes: Vec::new(),
                resource_type: "KeyframeStore<MomentsEventKeyframe>".to_string(),
                resource_version: "2.0".to_string(),
            },
            name: name.to_string(),
            playback: 1,
            playback_speed: 30.0,
            playback_speed_type: 0,
            resource_type: "GMSequence".to_string(),
            resource_version: "2.0".to_string(),
            show_backdrop: true,
            show_backdrop_image: false,
            time_units: 1,
            tracks: vec![track],
            visible_range: None,
            volume: 1.0,
            xorigin: 0,
            yorigin: 0,
        };

        let layer = GMImageLayer {
            gmimage_layer: String::new(),
            name_field: layer_guid.to_string(),
            blend_mode: 0,
            display_name: "default".to_string(),
            is_locked: false,
            name: layer_guid.to_string(),
            opacity: 100.0,
            resource_type: "GMImageLayer".to_string(),
            resource_version: "2.0".to_string(),
            visible: true,
        };

        Self {
            gmsprite: "v2".to_string(),
            name_field: name.to_string(),
            bbox_mode: 0,
            bbox_bottom: bbox.bottom,
            bbox_left: bbox.left,
            bbox_right: bbox.right,
            bbox_top: bbox.top,
            collision_kind: 1,
            collision_tolerance: 0,
            dynamic_texture_page: false,
            edge_filtering: false,
            for_3d: false,
            frames,
            grid_x: 0,
            grid_y: 0,
            height,
            h_tile: false,
            layers: vec![layer],
            name: name.to_string(),
            nine_slice: None,
            origin: 0,
            parent,
            pre_multiply_alpha: false,
            resource_type: "GMSprite".to_string(),
            resource_version: "2.0".to_string(),
            sequence,
            swatch_colours: None,
            swf_precision: 0.5,
            texture_group_id: ResourceReference {
                name: "Default".to_string(),
                path: "texturegroups/Default".to_string(),
            },
            sprite_type: 0,
            v_tile: false,
            width,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GMSpriteModel {
    #[serde(rename = "$GMSprite")]
    pub gmsprite: String,

    #[serde(rename = "%Name")]
    pub name_field: String,

    pub bbox_mode: i32,

    #[serde(rename = "bbox_bottom")]
    pub bbox_bottom: i32,

    #[serde(rename = "bbox_left")]
    pub bbox_left: i32,

    #[serde(rename = "bbox_right")]
    pub bbox_right: i32,

    #[serde(rename = "bbox_top")]
    pub bbox_top: i32,

    pub collision_kind: i32,
    pub collision_tolerance: i32,

    #[serde(rename = "DynamicTexturePage")]
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

    #[serde(rename = "Keyframes")]
    pub keyframes: Vec<serde_json::Value>,
    pub resource_type: String,
    pub resource_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MomentsEventKeyframeStore {
    #[serde(rename = "$KeyframeStore<MomentsEventKeyframe>")]
    pub keyframe_store: String,

    #[serde(rename = "Keyframes")]
    pub keyframes: Vec<serde_json::Value>,
    pub resource_type: String,
    pub resource_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteFrameKeyframeStore {
    #[serde(rename = "$KeyframeStore<SpriteFrameKeyframe>")]
    pub keyframe_store: String,

    #[serde(rename = "Keyframes")]
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

    #[serde(rename = "Channels")]
    pub channels: KeyframeChannels,

    #[serde(rename = "Disabled")]
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

    #[serde(rename = "Id")]
    pub id: ResourceReference,
    pub resource_type: String,
    pub resource_version: String,
}
