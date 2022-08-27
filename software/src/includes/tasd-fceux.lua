
--local inspect = require("inspect")
local api = require("tasd-api")

local movie_loaded = false
local handle = nil

local skip_lag = true
local frame = 0

local playerKeys = {
    {"right", "left", "down", "up", "start", "select", "B", "A"},
    {"right", "left", "down", "up", "start", "select", "B", "A"}
}

function writeFrame()
    local input = {}
    input[1] = joypad.get(1)
    input[2] = joypad.get(2)
    
    local chunk = {0, 0}
    
    for i = 1, 2 do
        for k = 7, 0, -1 do
            local key = playerKeys[i][k + 1]
            if input[i][key] == true then
                chunk[i] = bit.bor(chunk[i], bit.lshift(1, k))
            end
        end
        api.inputChunks(handle, i, { bit.bxor(chunk[i], 0xFF) })
    end
    
end

emu.speedmode("turbo")

while (true) do
    if movie.active() == true then
        if movie_loaded == false then
            movie.playbeginning()
            
            local movie_filename = movie.getname()
            local output_filename = string.sub(movie_filename, 0, #movie_filename - 4) .. ".tasd"
            
            print("Writing to " .. output_filename)
            
            handle = io.open(output_filename, "wb+")
            
            if (handle == nil) then
                print("Error opening dump file!")
            else
                api.header(handle)
                api.consoleType(handle, 1)
                api.emulatorName(handle, "FCEUX")
                api.dumpLastModified(handle)
                api.totalFrames(handle)
                api.rerecords(handle)
                api.blankFrames(handle, 0)
                api.portController(handle, 1, 0x0101)
                api.portController(handle, 2, 0x0101)
                
                movie_loaded = true
                emu.unpause()
            end
        end
        
        
        if movie.framecount() > 0 and movie.mode() ~= "finished" then
            local input = {}
            input[1] = joypad.get(1)
            
            if emu.lagged() == true then
                --[[if ( input[1].A == true  and input[1].B == true    and input[1].select == true and input[1].start == true and input[1].up == true and input[1].down == true and input[1].left == true   and input[1].right == false ) then
                    print("Reset frame detected! #"..tostring(frame))
                    
                    api.inputChunks(handle, {0, 0})
                    api.transition(handle, frame, 0x01)
                    
                    frame = frame + 1
                end]]--
            else
                writeFrame();
                frame = frame + 1;
            end
        end
        
        
    end
    
    if movie_loaded == true and movie.mode() == "finished" then
        handle:close()
        
        print("DONE Frames written: "..tostring(frame))
        movie_loaded = false
        frame = 0
        movie.stop()
        emu.pause()
        emu.exit()
    end
    
    emu.frameadvance();
end
