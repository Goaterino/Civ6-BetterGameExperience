-- Vérifie la syntaxe de scripts d'interface de Civ6 hors jeu :
--   lua proxy/tests/check_havok.lua <fichier.lua>...
-- HavokScript accepte des annotations de type (`local x:number`, `function f(a:table)`) que Lua 5.4
-- refuse : on les retire, comme les tests des blocs du proxy, puis on compile sans exécuter.
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end

local failed = 0;
for _, path in ipairs(arg) do
	local text = ReadFile(path);
	for _, t in ipairs({ "table", "number", "string", "boolean", "object", "userdata", "ifunction", "function" }) do
		text = text:gsub("([%w_%)%]])%s*:%s*" .. t .. "%f[^%w_(]", "%1");
	end
	local chunk, err = load(text, "@" .. path);
	if chunk == nil then
		failed = failed + 1;
		print("ÉCHEC " .. err);
	else
		print("ok    " .. path);
	end
end
os.exit(failed == 0 and 0 or 1);
