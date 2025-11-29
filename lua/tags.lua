-- Print all tag names in an Aseprite file
-- Usage: aseprite --script tags.lua -- <path_to_file.aseprite>

local function printAllTags(filePath)
  -- Open the file if a path is provided
  if filePath then
    app.open(filePath)
  end

  local sprite = app.sprite
  if not sprite then
    print("Error: No sprite is open")
    return
  end

  local tags = sprite.tags
  if #tags == 0 then
    print("No tags found in the sprite")
    return
  end

  print("Tags in sprite:")
  for i, tag in ipairs(tags) do
    print(string.format("  %d. %s (frames %d-%d)", i, tag.name, tag.fromFrame.frameNumber, tag.toFrame.frameNumber))
  end
end

-- Get file path from command line arguments
local filePath = nil
filePath = app.params["filepath"]

printAllTags(filePath)
