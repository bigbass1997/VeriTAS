
local handle = nil

local was_movie_loaded = false
local ready_to_dump = false

local keys_0 = {"C", "B", "right", "left", "down", "up", "start", "A"}
local keys_1 = {"", "", "", "", "mode", "X", "Y", "Z"}

local xyz_mode = {false, false}

local last_data_read = {0x55, 0x55}
local last_data_write = {0xAA, 0xAA}
local valid_read = {false, false}
local read_counter = {0, 0};

local write_counter = {0, 0}
local last_input = {nil, nil}



local working_input = {{}, {}}
local select_state = {false, false}
local frame_read_counts = {}

package.loaded["tasd-api"] = nil
_G["tasd-api"] = nil
--local inspect = require("inspect")
local api = require("tasd-api")

--[[
function write_input(port)
	if ready_to_dump then
		local input = joypad.get(port)
		--local input = last_input[port]
		local data = {0xFF, 0xFF}
		
		for k = 7, 0, -1 do
			local key = keys_0[k + 1]
			if input[key] == true then
				data[1] = XOR(data[1], BIT(k)) --bit.clear(data[1], k)
			end
		end
		
		if xyz_mode[port] then
			for k = 7, 4, -1 do
				local key = keys_1[k + 1]
				if input[key] == true then
					data[2] = XOR(data[2], BIT(k)) --bit.clear(data[2], k)
				end
			end
			
			api.inputChunks(handle, port, data)
		else
			api.inputChunks(handle, port, { data[1] })
			
			if data[1] ~= 0xFF then
				print(string.format("%u: %X", gens.framecount(), data[1]))
			end
		end
		
		last_dump_frame[port] = gens.framecount()
    end
end
]]--

function dump_input(port, valid_reads)
	if ready_to_dump then
		if xyz_mode[port] then
			print("6-button controllers are unsupported!")
		else
			for i = 1, #valid_reads, 2 do
				local high_state = 0xFF
				local low_state = 0xFF
				
				if valid_reads[i].state then
					high_state = AND(valid_reads[i].value, 0x3F)
					low_state = AND(valid_reads[i + 1].value, 0x3F)
				else
					high_state = AND(valid_reads[i + 1].value, 0x3F)
					low_state = AND(valid_reads[i].value, 0x3F)
				end
				
				local data = 0xFF
				
				for b = 5, 0, -1 do -- C, B, Right, Left, Down, Up
					if AND(high_state, BIT(b)) == 0 then
						data = XOR(data, BIT(5 - b))
					end
				end
				
				if AND(low_state, BIT(5)) == 0 then -- Start
					data = XOR(data, BIT(6))
				end
				
				if AND(low_state, BIT(4)) == 0 then -- A
					data = XOR(data, BIT(7))
				end
				
				api.inputChunks(handle, port, { data })
				if port == 1 then
					--print(string.format("%u: %X", gens.framecount(), data))
				end
			end
		end
    end
end

function dump_blank(port)
	if port == 1 then
		--print(string.format("%u: BLANK", gens.framecount()))
	end
	api.inputChunks(handle, port, { 0xFF })
end



-- read data port 1
memory.registerread(0xA10003, 1, function (addr, size, val)
    table.insert(working_input[1], {state = select_state[1], value = val})
end)

-- read data port 2
memory.registerread(0xA10005, 1, function (addr, size, val)
    table.insert(working_input[2], {state = select_state[2], value = val})
end)


-- write data port 1
memory.registerwrite(0xA10003, 1, function (addr, size, value)
	select_state[1] = AND(value, 0x40) > 0
end)

-- write data port 2
memory.registerwrite(0xA10005, 1, function (addr, size, value)
    select_state[2] = AND(value, 0x40) > 0
end)


local last_after_frame = -5
gens.registerafter(function ()
	if gens.framecount() == last_after_frame then
		return
	end
	last_after_frame = gens.framecount()
	
	for port = 1, 2 do
		local valid_reads = {}
		for i = 1, #working_input[port] do --don't use pairs, this must be performed in order
			local input = working_input[port][i]
			
			if #valid_reads == 0 then
				table.insert(valid_reads, input)
			elseif valid_reads[i - 1].state ~= input.state then
				table.insert(valid_reads, input)
			end
		end
		
		frame_read_counts[gens.framecount()] = #valid_reads
		
		if #valid_reads == 2 or #valid_reads > 4 then
			dump_input(port, valid_reads)
		elseif #valid_reads == 4 and frame_read_counts[gens.framecount() - 1] == 0 then
			dump_blank(port)
			dump_input(port, { valid_reads[3], valid_reads[4] })
		end
		
		working_input[port] = {}
	end
end)

function get_dump_filename()
    local _, _, path, filename, ext = string.find(movie.name(), "(.-)([^\\/]-%.?)([^%.\\/]*)$")
    return path..filename.."tasd"
end


while not movie.active() do
	gens.emulateframefast()
end

gens.speedmode("turbo")

while true do
	local goto_continue = false
	
	if movie.active() and not was_movie_loaded then
		if gens.framecount() == 0 then
			was_movie_loaded = true
			
			local filename = get_dump_filename()
			handle = io.open(filename, "wb+")
			
			if handle == nil then
				print("Error opening dump file!")
				break
			else
				-- figure out how to determine if a controller is 3-button or 6-button
				--if is_6_button then
				--    xyz_mode[port] = true
				--end
				
				api.header(handle)
                api.consoleType(handle, 8)
                api.emulatorName(handle, "Gens")
                api.dumpLastModified(handle)
                api.totalFrames(handle)
                api.rerecords(handle)
                api.blankFrames(handle, 0)
                for i = 1, 2 do
					if xyz_mode[i] then
						api.portController(handle, i, 0x0802)
					else
						api.portController(handle, i, 0x0801)
					end
                end
                handle:flush()
			end
			
			ready_to_dump = true
			print("Dumping has started...")
			movie.replay()
		elseif gens.framecount() > 0 then
			movie.replay()
			
			goto_continue = true
		end
	elseif not movie.active() then
		was_movie_loaded = false
		ready_to_dump = false
	end
	
	if not goto_continue then
		if movie.mode() == "finished" or (was_movie_loaded and movie.mode() == "record") or (was_movie_loaded and movie.mode() == nil) then
			was_movie_loaded = false
			ready_to_dump = false
			gens.pause()
			movie.stop()
			handle:close()
			print("Movie dump complete!")
			os.exit()
			break
		end
	end
	
	-- continue
	gens.frameadvance()
end