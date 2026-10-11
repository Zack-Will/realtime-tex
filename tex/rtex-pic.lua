-- rtex-pic.lua: picture cache mechanics shared by the background capture (rtex-capture.lua)
-- and the live server (rtex-serve.lua). A picture environment (tikzpicture, circuitikz,
-- pgfpicture) whose begin macro is wrapped (rtex-pic.tex) either runs as usual or, when the
-- caller's lookup finds a cache entry for it, has its body skipped and an image of the
-- recorded region of an earlier pass's PDF put in its place: exactly the picture's width,
-- height and depth, lowered by its depth, so placements do not change.
local P = {}

-- The environments handled (the same list as piccache::PICTURE_ENVS in Rust).
P.PICTURE_ENVS = { "tikzpicture", "circuitikz", "pgfpicture" }

-- State the picture inherits from its surroundings and that its text does not show: the
-- current font (by name and size: ids are allocation order, different from run to run) and
-- color, the line width (pgfplots `width=\linewidth`). A cached picture is used only when it
-- was drawn under the same state.
function P.state()
  local lw = ""
  pcall(function() lw = tostring(tex.dimen.linewidth) end)
  local f = font.current()
  local name = tex.fontname(f) or tostring(f)
  return name .. "/" .. tostring(token.get_macro("current@color") or "") ..
         "/" .. tostring(tex.hsize) .. "/" .. lw
end

-- Image objects per (pdf, page, bbox): scanning the PDF once per region, not per use.
local images = {}
function P.image(e)
  local key = tostring(e.pdf) .. ":" .. tostring(e.page) .. ":" ..
              tostring(e.bbox[1]) .. "," .. tostring(e.bbox[2]) .. "," .. tostring(e.bbox[3]) .. "," .. tostring(e.bbox[4])
  local im = images[key]
  if im then return im end
  im = img.new{ filename = e.pdf, page = e.page, bbox = { e.bbox[1], e.bbox[2], e.bbox[3], e.bbox[4] } }
  im = img.scan(im)
  images[key] = im
  return im
end

-- A controller: one per document (capture) or server. `cct` is the catcode table the printed
-- tokens use (LaTeX's, @ a letter).
function P.new(cct)
  return { cct = cct, armed = nil, pending = nil }
end

-- env/<env>/begin hook: the wrapper acts only for \begin{<env>}, not for a direct call of the
-- begin macro (\tikz, pgfplots' inner picture).
function P.arm(c, env)
  c.armed = env
end

local function orig(c, env)
  tex.sprint(c.cct, "\\csname rtex@orig@" .. env .. "\\endcsname")
end

-- The wrapped begin macro. `lookup(env, state)` returns the cache entry to use ({pdf, page,
-- bbox, d, …}) or nil; `on_draw(env, state)` runs when the picture is drawn. Returns true when
-- the cached image replaces the picture.
function P.begin(c, env, lookup, on_draw)
  if c.armed ~= env then
    orig(c, env)
    return false
  end
  c.armed = nil
  local state = P.state()
  local e = lookup(env, state)
  -- img.new/img.scan on a missing or unreadable file is a fatal TeX error that no pcall
  -- catches: the file is checked first (the cache may have evicted it meanwhile)
  if e and not (e.pdf and lfs.attributes(e.pdf, "mode") == "file") then e = nil end
  if e then
    local ok, im = pcall(P.image, e)
    if ok and im then
      c.pending = { img = im, entry = e }
      token.set_macro("rtex@picdepth", tostring(e.d) .. "sp")
      tex.sprint(c.cct, "\\rtex@gobblesetup{" .. env .. "}")
      return true
    end
  end
  if on_draw then on_draw(env, state) end
  orig(c, env)
  return false
end

-- Inside \hbox{...} of \rtex@picreplace: the cached picture's image node. `cached[index] = true`
-- marks the image resource as a cached picture for the display-list traversal.
function P.write(c, cached)
  local p = c.pending
  c.pending = nil
  if not p then return end
  local n = img.node(p.img)
  if n.index and cached then cached[n.index] = true end
  node.write(n)
end

-- TeX's image resource number is a PDF object number; image-rule nodes carry
-- the image array index instead. Make an unshipped node to obtain that index.
-- This also works when graphicx reuses an earlier resource from its cache.
function P.record_image(c, namespace, index, file, page, pages)
  c.image_pending = { file = file, page = tonumber(page) or 1, pages = pages }
  local hash = token.get_macro("Gin@attr@hash") or ""
  local command = token.get_macro(file .. " image" .. hash) or ("\\useimageresource " .. index)
  tex.sprint(c.cct, "\\begingroup\\setbox0=\\hbox{" .. command ..
    "}\\directlua{" .. namespace .. ".image_from_box()}\\endgroup")
end
function P.image_from_box(c)
  local n = tex.box[0] and node.tail(tex.box[0].list)
  local info = c.image_pending
  c.image_pending = nil
  if n and info then
    info.index = n.index
    c.images[tostring(n.index)] = info
  end
end

return P
