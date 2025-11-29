-- Export all tags from an Aseprite file
-- Usage: aseprite -b -script-param filepath=<path> -script-param outputdir=<dir> -script export_tags.lua

local function toCamelCase(str)
  -- Convert string to camelCase (capitalize first letter of each word)
  if not str or str == "" then
    return ""
  end

  -- Split by common separators
  local parts = {}
  for part in string.gmatch(str, "[^_%-%. ]+") do
    -- Capitalize first letter of each part
    if #part > 0 then
      local capitalized = string.upper(string.sub(part, 1, 1)) .. string.lower(string.sub(part, 2))
      table.insert(parts, capitalized)
    end
  end

  if #parts == 0 then
    return ""
  end

  -- All parts should be capitalized (we'll prepend 's' separately)
  return table.concat(parts, "")
end

local function exportTags(filePath, outputDir)
  -- Open the file
  app.open(filePath)

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

  -- Get base filename without extension
  local baseName = string.match(filePath, "([^/\\]+)%.aseprite$")
  if not baseName then
    baseName = string.match(filePath, "([^/\\]+)$")
    baseName = string.gsub(baseName, "%.aseprite$", "")
  end

  local baseNameCamel = toCamelCase(baseName)

  -- Export each tag
  for i, tag in ipairs(tags) do
    -- Reopen the file to ensure we're working with the original
    app.open(filePath)
    sprite = app.sprite

    local tagNameCamel = toCamelCase(tag.name)
    local frameCount = tag.toFrame.frameNumber - tag.fromFrame.frameNumber + 1

    -- Always export as PNG sprite sheet
    local outputName = string.format("s%s%s.png", baseNameCamel, tagNameCamel)

    -- Build full output path
    local outputPath
    if string.sub(outputDir, -1) == "/" or string.sub(outputDir, -1) == "\\" then
      outputPath = outputDir .. outputName
    else
      outputPath = outputDir .. "/" .. outputName
    end

    -- Get sprite dimensions (canvas size)
    local spriteWidth = sprite.width
    local spriteHeight = sprite.height

    -- Use Aseprite's built-in ExportSpriteSheet with tag parameter
    -- Reference: https://www.aseprite.org/api/command/ExportSpriteSheet#exportspritesheet
    app.command.ExportSpriteSheet {
      ui = false,
      type = SpriteSheetType.HORIZONTAL,
      columns = 0,       -- Auto-calculate columns
      rows = 0,          -- Auto-calculate rows
      textureFilename = outputPath,
      dataFilename = "", -- No JSON data file
      filenameFormat = outputName,
      borderPadding = 0,
      shapePadding = 0,
      innerPadding = 0,
      trimSprite = false,
      trim = false,
      extrude = false,
      ignoreEmpty = false,
      mergeDuplicates = false,
      openGenerated = false,
      tag = tag.name, -- Export only frames in this tag
      splitTags = false,
      listTags = false
    }

    -- Output JSON with sprite information for Rust to parse
    -- Output to stderr to avoid mixing with Aseprite's stdout JSON
    -- Properly escape the path and tag name for JSON
    local escapedPath = string.gsub(outputPath, "\\", "\\\\")
    escapedPath = string.gsub(escapedPath, '"', '\\"')
    local escapedTagName = string.gsub(tag.name, '"', '\\"')

    -- Output to stderr (io.stderr) so it doesn't mix with Aseprite's JSON output
    io.stderr:write(string.format('JSON_EXPORT:{"path":"%s","width":%d,"height":%d,"frame_count":%d,"tag_name":"%s"}\n',
      escapedPath, spriteWidth, spriteHeight, frameCount, escapedTagName))
  end
end

-- Get parameters
local filePath = app.params["filepath"]
local outputDir = app.params["outputdir"]

if not filePath then
  print("Error: filepath parameter is required")
  return
end

if not outputDir then
  -- Default to same directory as file
  outputDir = string.match(filePath, "^(.*)[/\\]")
  if not outputDir then
    outputDir = "."
  end
end

exportTags(filePath, outputDir)
