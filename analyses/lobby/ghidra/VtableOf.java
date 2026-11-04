// Pour chaque fonction donnée, trouve les tables (vtables) qui la référencent et liste leurs entrées.
// Arguments : <fichier de sortie> <adresse1> [adresse2 ...]
// Sortie : pour chaque référence de données, les 24 pointeurs à partir du début présumé de la table
// (premier pointeur précédé d'une donnée non-fonction), avec le nom RTTI éventuel.
//@category Civ6
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.mem.*;
import ghidra.program.model.symbol.*;
import java.io.*;

public class VtableOf extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] a = getScriptArgs();
        PrintWriter w = new PrintWriter(new OutputStreamWriter(new FileOutputStream(a[0], true), "UTF-8"));
        Memory mem = currentProgram.getMemory();
        FunctionManager fm = currentProgram.getFunctionManager();
        ReferenceManager rm = currentProgram.getReferenceManager();
        for (int i = 1; i < a.length; i++) {
            Address f = toAddr(a[i]);
            w.println("## " + f);
            for (Reference r : rm.getReferencesTo(f)) {
                if (r.getReferenceType().isCall() || r.getReferenceType().isJump()) continue;
                Address at = r.getFromAddress();
                if (fm.getFunctionContaining(at) != null) continue;
                // remonte jusqu'au début de la table
                Address start = at;
                for (int k = 0; k < 200; k++) {
                    Address prev = start.subtract(8);
                    long v = mem.getLong(prev);
                    Function pf = fm.getFunctionAt(toAddr(v));
                    if (pf == null) break;
                    start = prev;
                }
                Symbol s = getSymbolAt(start);
                Address meta = start.subtract(8);
                w.println("table " + start + " (ref en " + at + ", index " + (at.subtract(start) / 8) + ") " + (s == null ? "" : s.getName(true)));
                for (int k = 0; k < 40; k++) {
                    Address e = start.add(8L * k);
                    long v = mem.getLong(e);
                    Function ef = fm.getFunctionAt(toAddr(v));
                    if (ef == null) break;
                    w.println("  +" + Integer.toHexString(8 * k) + "\t" + ef.getEntryPoint() + "\t" + ef.getName());
                }
            }
        }
        w.close();
    }
}
