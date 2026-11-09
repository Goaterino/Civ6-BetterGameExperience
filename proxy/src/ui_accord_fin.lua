-- ===========================================================================
-- Civ6RolloverProxy (src/ui_accord_fin.lua), ajouté par le proxy à la fin de DiplomacyDealView.lua
-- (MPH inclut ce fichier) : pas d'accord envoyé à un humain pendant le passage de tour.
-- Partie du 2026-11-09 (depart_tour/README.md, « Accord pendant le passage de tour ») : un accord
-- proposé ou accepté entre deux humains dans la dernière seconde du tour arrive chez l'hôte après
-- `Game Turn Ended`. L'hôte lance alors le tour suivant avant d'avoir reçu le `GameTurnComplete` de
-- certains joueurs. Ces joueurs sont marqués « tour fini » dès l'activation, sans
-- `PlayerTurnComplete`, et chaque annulation est défaite aussitôt : tours passés en boucle jusqu'au
-- relobby, avec une désynchro au contrôle suivant. Les deux seuls accords entre humains arrivés
-- pendant un passage de tour ont donné les deux seuls tours bloqués.
-- On enveloppe donc DealManager.SendWorkingDeal (proposition, acceptation, refus, exigence) dans
-- ce contexte : vers un humain, en multijoueur en réseau, l'accord ne part pas
--   - si le minuteur du tour local est écoulé (fin forcée par l'hôte comprise) ;
--   - ou si plus aucun humain n'a son tour actif (le passage de tour a commencé).
-- Les deux accords en cause sont partis après la fin du tour de leur expéditeur : son
-- PlayerTurnComplete arrive avant eux chez l'hôte, minuteur déjà écoulé. Un accord envoyé pendant
-- que l'on joue arrive avant notre PlayerTurnComplete (messages d'un joueur dans l'ordre), donc
-- avant la fin du tour : rien à retenir. Pas de marge avant la fin du minuteur, donc : elle
-- bloquerait les accords de la dernière seconde pour rien.
-- Le joueur est prévenu dans le chat (ligne locale, ui_depart_chat.lua) et propose au tour suivant.
-- Les accords avec les IA ne changent pas : leurs réponses partent aussi pendant le passage de tour.
-- ===========================================================================
do
	local AccordFin_Ok, AccordFin_Err = pcall(function()
		if BGD_AccordFin ~= nil or DealManager == nil or DealManager.SendWorkingDeal == nil then
			return;
		end
		BGD_AccordFin = true;

		local DealManager_jeu = DealManager;
		local SendWorkingDeal_jeu = DealManager_jeu.SendWorkingDeal;

		local function Reseau()
			return GameConfiguration.IsNetworkMultiplayer() and not GameConfiguration.IsPlayByCloud()
				and not GameConfiguration.IsHotseat();
		end

		local function Humain(id)
			local pc = PlayerConfigurations[id];
			return pc ~= nil and pc:IsHuman();
		end

		-- Un humain vivant a-t-il encore son tour actif ?
		local function TourEnCours()
			for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
				local p = Players[id];
				if Humain(id) and p ~= nil and p:IsAlive() and p:IsTurnActive() then
					return true;
				end
			end
			return false;
		end

		-- Pourquoi l'accord ne doit pas partir maintenant, ou nil.
		local function Raison(moi)
			local ok, elapsed = pcall(UI.IsTurnTimerElapsed, moi);
			if ok and elapsed == true then
				return "minuteur écoulé";
			end
			if not TourEnCours() then
				return "passage de tour";
			end
			return nil;
		end

		local function Bloque(eAction, de, vers)
			if not Reseau() or vers == nil or vers == de or not Humain(vers) then
				return nil;
			end
			return Raison(de);
		end

		local function SendWorkingDeal(eAction, de, vers, ...)
			local ok, raison = pcall(Bloque, eAction, de, vers);
			if ok and raison ~= nil then
				print("BGE accord : non envoyé au joueur " .. tostring(vers) .. " (" .. raison .. ")");
				pcall(function()
					LuaEvents.BGD_DepartTexte("[COLOR_Civ6Yellow]BGE - " .. BGD_Txt("LOC_BGD_ACCORD_FIN_TOUR") .. "[ENDCOLOR]");
				end);
				return;
			end
			return SendWorkingDeal_jeu(eAction, de, vers, ...);
		end

		-- Le reste de DealManager est celui du jeu : seule cette globale du contexte change.
		DealManager = setmetatable({ SendWorkingDeal = SendWorkingDeal }, { __index = DealManager_jeu });
	end);
	if not AccordFin_Ok then
		print("BGE accord fin de tour : echec " .. tostring(AccordFin_Err));
	end
end
