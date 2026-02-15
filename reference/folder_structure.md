GameMaker core folder structure for all projects:
datafiles [folder]
options [folder]
rooms [folder]
sprites [folder]
.gitattributes
.gitignore
[NameOfProject].resource_order
[NameOfProject].yyp

## Folder structure for sprites in GameMaker:

INSIDE of the default 'sprites' folder:

example name of sprite: sTestSprite

there exists a folder sprites/sTestSprite

### INSIDE the sTestSprite folder:

layers [folder]
sTestSprite.yy
FOREACH subimage of the sprite:
image is assigned a GUID and a '.png' extension

### FOR EXAMPLE if sTestSprite had three subimages:

layers [folder]
sTestSprite.yy
9bfcd653-f5bf-4dc2-8fe8-fa2639642544.png
69bdae94-4166-494b-aeaa-4e041772b216.png
b52181f8-ee8f-446a-b69f-bd2f3209223e.png

### INSIDE the 'layers' folder (Using the same example):

9bfcd653-f5bf-4dc2-8fe8-fa2639642544 [folder]
69bdae94-4166-494b-aeaa-4e041772b216 [folder]
b52181f8-ee8f-446a-b69f-bd2f3209223e [folder]

#### INSIDE folder 9bfcd653-f5bf-4dc2-8fe8-fa2639642544 (The first one):

e38d3a3b-b24b-4d7f-ac4b-45fe93f1aa04.png

- This '.png' will be the same exact image of 9bfcd653-f5bf-4dc2-8fe8-fa2639642544.png
- Note that this GUID is the name of the layer the image belongs to. All subimages for Aseprite exports should have the same layer

#### INSIDE folder 69bdae94-4166-494b-aeaa-4e041772b216 (The second one):

e38d3a3b-b24b-4d7f-ac4b-45fe93f1aa04.png

- This '.png' will be the same exact image of 69bdae94-4166-494b-aeaa-4e041772b216.png

#### INSIDE folder b52181f8-ee8f-446a-b69f-bd2f3209223e (The third one):

e38d3a3b-b24b-4d7f-ac4b-45fe93f1aa04.png

- This '.png' will be the same exact image of b52181f8-ee8f-446a-b69f-bd2f3209223e.png

- Take note of the sample gamemaker_sprite_yy.json to see how everything is laid out

## Defining folders for the project

- For GameMaker projects, folders are defined in the .yyp like this:
  "Folders":[
  {"$GMFolder":"","%Name":"Sprites","folderPath":"folders/Sprites.yy","name":"Sprites","resourceType":"GMFolder","resourceVersion":"2.0"},
  {"$GMFolder":"","%Name":"NestedFolder","folderPath":"folders/Sprites/NestedFolder.yy","name":"NestedFolder","resourceType":"GMFolder","resourceVersion":"2.0"}
  ],
- Take note of the gamemaker_yyp.json to understand the structure
