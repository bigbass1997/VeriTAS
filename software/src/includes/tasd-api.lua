local api = {}

local char = string.char

local MAGIC_NUMBER  = char(0x54, 0x41, 0x53, 0x44)
local TASD_VERSION  = char(0x00, 0x01)
local KEY_WIDTH     = char(0x02)

local CONSOLE_TYPE      = char(0x00, 0x01)
local CONSOLE_REGION    = char(0x00, 0x02)
local GAME_TITLE        = char(0x00, 0x03)
local ROM_NAME          = char(0x00, 0x04)
local ATTRIBUTION       = char(0x00, 0x05)
local CATEGORY          = char(0x00, 0x06)
local EMULATOR_NAME     = char(0x00, 0x07)
local EMULATOR_VERSION  = char(0x00, 0x08)
local EMULATOR_CORE     = char(0x00, 0x09)
local TAS_LAST_MODIFIED = char(0x00, 0x0A)
local DUMP_CREATED      = char(0x00, 0x0B)
local DUMP_LAST_MODIFIED= char(0x00, 0x0C)
local TOTAL_FRAMES      = char(0x00, 0x0D)
local RERECORDS         = char(0x00, 0x0E)
local SOURCE_LINK       = char(0x00, 0x0F)
local BLANK_FRAMES      = char(0x00, 0x10)
local VERIFIED          = char(0x00, 0x11)
local MEMORY_INIT       = char(0x00, 0x12)
local GAME_IDENTIFIER   = char(0x00, 0x13)
local MOVIE_LICENSE     = char(0x00, 0x14)
local MOVIE_FILE        = char(0x00, 0x15)

local PORT_CONTROLLER   = char(0x00, 0xF0)

local NES_LATCH_FILTER  = char(0x01, 0x01)
local NES_CLOCK_FILTER  = char(0x01, 0x02)
local NES_OVERREAD      = char(0x01, 0x03)
local NES_GAME_GENIE_CODE= char(0x01, 0x04)

local SNES_CLOCK_FILTER = char(0x02, 0x02)
local SNES_OVERREAD     = char(0x02, 0x03)
local SNES_GAME_GENIE_CODE= char(0x02, 0x04)

local GENESIS_GAME_GENIE_CODE= char(0x08, 0x04)

local INPUT_CHUNK       = char(0xFE, 0x01)
local INPUT_MOMENT      = char(0xFE, 0x02)
local TRANSITION        = char(0xFE, 0x03)
local LAG_FRAME_CHUNK   = char(0xFE, 0x04)
local MOVIE_TRANSITION  = char(0xFE, 0x05)
local COMMENT           = char(0xFF, 0x01)
local UNSPECIFIED       = char(0xFF, 0xFF)

function calcExponent(number)
    local exp = 0
    local n = number
    while n ~= 0 do
        n = bit.rshift(n, 8)
        exp = exp + 1
    end
    
    if exp == 0 then
        exp = 1
    end
    
    return exp
end

function packet(h, key, strPayload)
    local exp = calcExponent(#strPayload)
    local length = encodeNumber(#strPayload, exp)
    h:write(key..char(exp)..length..strPayload)
end

-- turn a number into a string of a specified number of char'ed bytes
function encodeNumber(value, byteLength)
    local s = ""
    for i = 1, byteLength do
        s = s..char(bit.band(value, 0xFF))
        value = bit.rshift(value, 8)
    end
    return string.reverse(s)
end


function api.header(h)
    h:write(MAGIC_NUMBER..TASD_VERSION..KEY_WIDTH)
end

-- custom is required only if kind == 0xFF, it should be a string
function api.consoleType(h, kind, custom)
    if kind == 0xFF then
        packet(h, CONSOLE_TYPE, char(kind)..custom)
    else
        packet(h, CONSOLE_TYPE, char(kind))
    end
end

-- region = a number from 0x00 to 0xFF
function api.consoleRegion(h, region)
    packet(h, CONSOLE_REGION, char(region))
end

-- title = string
function api.gameTitle(h, title)
    packet(h, GAME_TITLE, title)
end

-- name = string
function api.romName(h, name)
    packet(h, ROM_NAME, name)
end

-- author = string
function api.author(h, author)
    packet(h, ATTRIBUTION, encodeNumber(0x01, 1)..author)
end

-- category = string
function api.category(h, category)
    packet(h, CATEGORY, category)
end

-- name = string
function api.emulatorName(h, name)
    packet(h, EMULATOR_NAME, name)
end

-- version = string
function api.emulatorVersion(h, version)
    packet(h, EMULATOR_VERSION, version)
end

-- core = string
function api.emulatorCore(h, core)
    packet(h, EMULATOR_CORE, core)
end

-- time = 8-byte signed number representing epoch (in seconds)
function api.tasLastModified(h, time)
    packet(h, TAS_LAST_MODIFIED, encodeNumber(time, 8))
end

function api.dumpLastModified(h)
    packet(h, DUMP_LAST_MODIFIED, encodeNumber(os.time(), 8))
end

function api.totalFrames(h)
    packet(h, TOTAL_FRAMES, encodeNumber(movie.length(), 4))
end

function api.rerecords(h)
    local count = 0
    if type(movie.getrerecordcount) == "function" then
        count = movie.getrerecordcount()
    elseif type(movie.rerecordcount) == "function" then
        count = movie.rerecordcount()
    end
    
    packet(h, RERECORDS, encodeNumber(count, 4))
end

-- url = string
function api.sourceLink(h, url)
    packet(h, SOURCE_LINK, url)
end

-- frames = 2-byte signed number
function api.blankFrames(h, frames)
    packet(h, BLANK_FRAMES, encodeNumber(frames, 2))
end

-- verified = either a number of 0 or 1, or a boolean
function api.verified(h, verified)
    if type(verified) == "boolean" then
        packet(h, VERIFIED, char(verified == true and 1 or verified == false and 0))
    else
        packet(h, VERIFIED, char(verified))
    end
end

-- TODO function api.memoryInit()

-- port = port number byte (1-indexed)
-- controllerType = 2-byte controller type number
function api.portController(h, port, controllerType)
    packet(h, PORT_CONTROLLER, char(port)..encodeNumber(controllerType, 2))
end

-- filter = number from 0x00 to 0xFF
function api.nesLatchFilter(h, filter)
    packet(h, NES_LATCH_FILTER, char(filter))
end

-- filter = number from 0x00 to 0xFF
function api.nesClockFilter(h, filter)
    packet(h, NES_CLOCK_FILTER, char(filter))
end

-- overread = either a number of 0 or 1, or a boolean
function api.nesOverread(h, overread)
    if type(overread) == "boolean" then
        packet(h, NES_OVERREAD, char(overread == true and 1 or overread == false and 0))
    else
        packet(h, NES_OVERREAD, char(overread))
    end
end

-- code == string (6 or 8 characters long)
function api.nesGameGenieCode(h, code)
    packet(h, NES_GAME_GENIE_CODE, code)
end

-- port = number from 0x01 to 0xFF (0x00 should never be used as a port number)
-- chunk = array of bytes (each byte is a number with a value from 0x00 to 0xFF)
function api.inputChunks(h, port, chunk)
    local payloadStr = char(port)
    for i = 1, #chunk do
        payloadStr = payloadStr..char(chunk[i])
    end
    
    packet(h, INPUT_CHUNK, payloadStr)
end

-- port = number from 0x01 to 0xFF (0x00 should never be used as a port number)
-- indexType = what this index represents (0x01 = frame, 0x02 = cycle count, 0x03 = milliseconds, 0x04 = microseconds * 10)
-- index = 8-byte unsigned index number (number from 0x00000000 to 0xFFFFFFFF)
-- input = array of bytes (each byte is a number with a value from 0x00 to 0xFF)
function api.inputMoment(h, port, indexType, index, input)
    local payloadStr = char(port)..char(indexType)..encodeNumber(index, 8)
    for i = 1, #input do
        payloadStr = payloadStr..char(input[i])
    end
    
    packet(h, INPUT_MOMENT, payloadStr)
end

-- Packet-derived transitions are NOT supported! You'll need to encode that case yourself.
-- index = 8-byte unsigned index number (number from 0x00000000 to 0xFFFFFFFF)
-- kind = type of transition (0x01 = soft reset, 0x02 = power reset, 0x03 = restart TASD file)
function api.transition(h, index, kind)
    packet(h, TRANSITION, encodeNumber(index, 8)..char(kind))
end

-- index = 4-byte unsigned frame index number, the start of the lag frame chunk (number from 0x00000000 to 0xFFFFFFFF)
-- count = 4-byte length of the chunk (number from 0x00000000 to 0xFFFFFFFF)
function api.lagFrameChunk(h, index, count)
    packet(h, LAG_FRAME_CHUNK, encodeNumber(index, 4)..encodeNumber(count, 4))
end

-- Packet-derived transitions are NOT supported! You'll need to encode that case yourself.
-- index = 4-byte unsigned frame index number (number from 0x00000000 to 0xFFFFFFFF)
-- kind = type of transition (number from 0x01 to 0xFF)
function api.movieTransition(h, index, kind)
    packet(h, MOVIE_TRANSITION, encodeNumber(index, 4)..char(kind))
end

















return api