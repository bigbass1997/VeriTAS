
local handle = nil

local wasMovieLoaded = false
local readyToDump = false

local writtenFrames = 0
local latch_count = 0;

local playerKeys = {
    {"P1 Right", "P1 Left", "P1 Down", "P1 Up", "P1 Start", "P1 Select", "P1 B", "P1 A"},
    {"P2 Right", "P2 Left", "P2 Down", "P2 Up", "P2 Start", "P2 Select", "P2 B", "P2 A"}
}

package.loaded["tasd-api"] = nil
_G["tasd-api"] = nil
--local inspect = require("inspect")
local api = require("tasd-api")

console.clear()

function getDumpFilename()
    local _, _, path, filename, ext = string.find(movie.filename(), "(.-)([^\\/]-%.?)([^%.\\/]*)$")
    return path..filename.."tasd"
end

function writeFrame()
    local input = movie.getinput(emu.framecount() - 1)
    
    local data = {0xFF, 0xFF}
    
    for i = 1, 2 do
        for k = 7, 0, -1 do
            local key = playerKeys[i][k + 1]
            if input[key] == true then
                data[i] = bit.clear(data[i], k)
            end
        end
        
        api.inputChunks(handle, i, { data[i] })
    end
    
    latch_count = latch_count + 1
    --print("("..emu.framecount()..") "..writtenFrames.." is "..string.format("0x%02X 0x%02X", chunk[1], chunk[2]))
end

while not movie.isloaded() do
    emu.yield()
end

while true do
    if movie.isloaded() and not wasMovieLoaded then
        if emu.framecount() == 0 then
            wasMovieLoaded = true
            local filename = getDumpFilename()
            handle = io.open(filename, "wb+")
            
            if handle == nil then
                print("Error opening dump file!")
                break
            else
                api.header(handle)
                api.consoleType(handle, 1)
                api.emulatorName(handle, "Bizhawk")
                api.dumpLastModified(handle)
                api.totalFrames(handle)
                api.rerecords(handle)
                api.blankFrames(handle, 0)
                api.portController(handle, 1, 0x0101)
                api.portController(handle, 2, 0x0101)
                handle:flush()
            end
            
            --print(tostring(movie.length()-1)..": "..inspect(movie.getinput(movie.length()-1)))
            
            print("Dumping has started...")
            --[[print("on frame: "..emu.framecount())
            print("Lag frames on start: "..emu.lagcount())]]--
            client.unpause()
            readyToDump = true
            writtenFrames = 0
        elseif emu.framecount() > 0 then
            client.pause()
            print("Sorry! You cannot activate/start this script after the first frame of a movie!")
            print("Use: File > Movie > Play from Beginning")
            print("Then while the movie is still paused, activate this script again.")
            break
        end
    elseif not movie.isloaded() then
        wasMovieLoaded = false
        readyToDump = false
    end
    
    
    if readyToDump then
        if emu.framecount() <= movie.length() and emu.framecount() > 0 then
            local input = movie.getinput(emu.framecount() - 1)
            if input["Reset"] == true then
                print("Soft Reset on frame: "..(emu.framecount() - 1).." | latch: "..latch_count)
                api.transition(handle, 0x05, latch_count * 2, 0x01)
            elseif input["Power"] == true then
                print("Power Reset on frame: "..(emu.framecount() - 1))
                api.transition(handle, 0x05, latch_count * 2, 0x02)
            end
        end
        
        if not emu.islagged() and emu.framecount() <= movie.length() and emu.framecount() > 0 then
            --print(emu.framecount()..": "..inspect(movie.getinput(emu.framecount())))
            --table.insert(allInputs, movie.getinput(emu.framecount()))
            
            writeFrame()
            handle:flush()
            writtenFrames = writtenFrames + 1
        end
        
        --[[if emu.islagged() then
            print("("..emu.framecount()..") "..writtenFrames.." is lag")
        end
        print("Lag frames: "..emu.lagcount())]]--
    end
    
    
    if movie.mode() == "FINISHED" or (wasMovieLoaded and movie.mode() == "RECORD") then
        wasMovieLoaded = false
        readyToDump = false
        client.pause()
        movie.stop()
        handle:close()
        print("Movie dump complete!")
        client.exit()
        break
    end
    
    emu.frameadvance()
end
