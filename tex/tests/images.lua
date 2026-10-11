-- Check the node-index -> source map after two graphics loads and a cache hit.
local json = dofile("tex/rtex-json.lua")
local function read(path)
  local f = assert(io.open(path)); local s = f:read("*a"); f:close(); return json.decode(s)
end
local page = read(arg[1] .. "/main.rtex-page1.json")
local files = {}
for _, line in ipairs(page.lines) do
  for _, item in ipairs(line.items) do
    if item[1] == "i" then
      local info = assert(page.images_info[tostring(item[2])], "image has no source")
      files[#files + 1] = info.file
    end
  end
end
assert(#files == 3 and files[1] == "a.png" and files[2] == "b.png" and files[3] == "a.png",
       "wrong image source after graphicx cache hit")
print("ok   image node indices resolve to a.png, b.png, a.png")
print("all image tests passed")
