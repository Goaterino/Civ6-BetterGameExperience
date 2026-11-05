-- ===========================================================================
-- Civ6RolloverProxy (src/ui_diag_accords.lua) : diagnostic des accords croisés entre humains (voir
-- docs/netcode/README.md, « Accords croisés »). Avec `ui_diag_accords=on`, le proxy l'ajoute à la
-- fin de DiplomacyDealView.lua, contexte toujours chargé, même écran fermé.
-- Il ne fait qu'écrire dans Lua.log, lignes `BGD accords …` :
-- - chaque événement d'accord ou de session qui touche le joueur local (DiplomacyIncomingDeal,
--   DiplomacyStatement, DiplomacySessionClosed, DiplomacyDealEnacted) ;
-- - chaque ouverture et fermeture de l'écran d'accord ;
-- - à chaque fois, l'état vu par l'écran : session ouverte et son initiateur, HasPendingDeal dans
--   les deux sens, contenu des accords entrant et sortant, dernière action reçue, bouton Accepter.
-- La question : la proposition d'un humain arrive-t-elle (DiplomacyIncomingDeal) quand on a déjà sa
-- propre session ouverte avec lui ? Si oui, le correctif peut s'appuyer sur cet événement.
-- Tout s'exécute sous pcall : un échec écrit `BGD accords echec <erreur>` et laisse l'écran intact.
-- ===========================================================================
do
local DiagAccords_Ok, DiagAccords_Err = pcall(function()
	if BGD_DiagAccords ~= nil then
		return;
	end
	BGD_DiagAccords = true;

	local actionNames = {};
	for k, v in pairs(DealProposalAction or {}) do
		actionNames[v] = k;
	end
	local function ActionName(a)
		if a == nil then return "nil"; end
		return tostring(actionNames[a] or a);
	end

	local function Try(f, ...)
		local ok, r = pcall(f, ...);
		if ok then return r; end
		return "err";
	end

	local function DealText(direction, me, other)
		local pDeal = DealManager.GetWorkingDeal(direction, me, other);
		if pDeal == nil then
			return "nil";
		end
		local mine = Try(function() return pDeal:GetItemCount(me, other); end);
		local theirs = Try(function() return pDeal:GetItemCount(other, me); end);
		local valid = Try(function() return pDeal:IsValid(); end);
		return string.format("%s+%s valide=%s", tostring(mine), tostring(theirs), tostring(valid));
	end

	-- Ce que l'écran verrait face à `other`. `ms_LastIncomingDealProposalAction` est un local du
	-- fichier hôte, visible ici puisque le bloc est compilé dans le même texte.
	local function State(other)
		local me = Game.GetLocalPlayer();
		if me == nil or me < 0 or other == nil or other < 0 or other == me then
			return "";
		end
		local session = DiplomacyManager.FindOpenSessionID(me, other);
		local from = "-";
		if session ~= nil then
			local info = DiplomacyManager.GetSessionInfo(session);
			from = tostring(info and info.FromPlayer);
		end
		local s = string.format(
			"session=%s de=%s attente(%d>%d)=%s attente(%d>%d)=%s entrant=%s sortant=%s",
			tostring(session), from,
			me, other, tostring(Try(DealManager.HasPendingDeal, me, other)),
			other, me, tostring(Try(DealManager.HasPendingDeal, other, me)),
			DealText(DealDirection.INCOMING, me, other), DealText(DealDirection.OUTGOING, me, other));
		if ms_OtherPlayerID == other and not ContextPtr:IsHidden() then
			s = s .. string.format(" ecran: initiateur=%s derniere=%s accepter=%s",
				tostring(ms_InitiatedByPlayerID), ActionName(ms_LastIncomingDealProposalAction),
				Controls.AcceptDeal:IsHidden() and "cache" or "visible");
		end
		return s;
	end

	local function Log(what, other)
		print(string.format("BGD accords %s | %s", what, State(other)));
	end

	local function Involved(a, b)
		local me = Game.GetLocalPlayer();
		return a == me or b == me;
	end
	local function OtherOf(a, b)
		if a == Game.GetLocalPlayer() then return b; end
		return a;
	end

	Events.DiplomacyIncomingDeal.Add(function(from, to, action)
		if Involved(from, to) then
			Log(string.format("entrant %s>%s %s", tostring(from), tostring(to), ActionName(action)), OtherOf(from, to));
		end
	end);

	Events.DiplomacyStatement.Add(function(from, to, kVariants)
		if not Involved(from, to) then
			return;
		end
		local parts = {};
		if type(kVariants) == "table" then
			for k, v in pairs(kVariants) do
				if type(v) ~= "table" and type(v) ~= "userdata" and type(v) ~= "function" then
					parts[#parts + 1] = tostring(k) .. "=" .. tostring(v);
				end
			end
			table.sort(parts);
		end
		Log(string.format("declaration %s>%s %s", tostring(from), tostring(to), table.concat(parts, ",")), OtherOf(from, to));
	end);

	Events.DiplomacySessionClosed.Add(function(sessionID)
		print(string.format("BGD accords session fermee %s | %s", tostring(sessionID),
			ms_OtherPlayerID ~= nil and ms_OtherPlayerID >= 0 and State(ms_OtherPlayerID) or ""));
	end);

	if Events.DiplomacyDealEnacted ~= nil then
		Events.DiplomacyDealEnacted.Add(function(...)
			local args = {};
			for i = 1, select("#", ...) do
				args[#args + 1] = tostring((select(i, ...)));
			end
			print("BGD accords accord execute " .. table.concat(args, ","));
		end);
	end

	-- Ouverture et fermeture : les fonctions sont relues à l'appel, pour suivre une surcharge posée
	-- plus tard par un autre fichier du contexte.
	ContextPtr:SetShowHandler(function()
		OnShow();
		Log("ouverture", ms_OtherPlayerID);
	end);
	ContextPtr:SetHideHandler(function(...)
		Log("fermeture", ms_OtherPlayerID);
		OnHide(...);
	end);
	print("BGD accords diagnostic pose");
end);
if not DiagAccords_Ok then
	print("BGD accords echec " .. tostring(DiagAccords_Err));
end
end
