// Décompile les fonctions qui citent des chaînes données, et leurs appelants / appelés directs.
// Arguments : <dossier de sortie> <profondeur 0|1> <motif1> [motif2 ...]
// Un motif est une sous-chaîne (sensible à la casse) cherchée dans les chaînes définies.
// Sortie : un fichier <adresse>.c par fonction, et index.tsv (adresse, nom, motif, rôle).
//@category Civ6
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;
import ghidra.program.model.data.StringDataInstance;
import java.io.*;
import java.util.*;

public class DumpByStrings extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] a = getScriptArgs();
        File out = new File(a[0]);
        out.mkdirs();
        int depth = Integer.parseInt(a[1]);
        List<String> pats = new ArrayList<>(Arrays.asList(a).subList(2, a.length));

        DecompInterface dec = new DecompInterface();
        dec.openProgram(currentProgram);
        Listing lst = currentProgram.getListing();
        ReferenceManager rm = currentProgram.getReferenceManager();
        FunctionManager fm = currentProgram.getFunctionManager();

        Map<Function, String> roles = new LinkedHashMap<>();
        PrintWriter idx = new PrintWriter(new FileWriter(new File(out, "index.tsv"), true));
        PrintWriter strs = new PrintWriter(new FileWriter(new File(out, "strings.tsv"), true));

        DataIterator it = lst.getDefinedData(true);
        while (it.hasNext() && !monitor.isCancelled()) {
            Data d = it.next();
            if (!d.hasStringValue()) continue;
            String s;
            try { s = StringDataInstance.getStringDataInstance(d).getStringValue(); } catch (Exception e) { continue; }
            if (s == null) continue;
            for (String p : pats) {
                if (!s.contains(p)) continue;
                for (Reference r : rm.getReferencesTo(d.getAddress())) {
                    Function f = fm.getFunctionContaining(r.getFromAddress());
                    String fn = f == null ? "-" : f.getEntryPoint().toString();
                    strs.println(d.getAddress() + "\t" + r.getFromAddress() + "\t" + fn + "\t" + p + "\t" + s.replace('\t', ' ').replace('\n', ' '));
                    if (f != null && !roles.containsKey(f)) roles.put(f, "cite:" + p);
                }
            }
        }
        strs.close();

        Set<Function> all = new LinkedHashSet<>(roles.keySet());
        if (depth > 0) {
            for (Function f : new ArrayList<>(roles.keySet())) {
                for (Function c : f.getCalledFunctions(monitor)) if (all.add(c)) roles.put(c, "appele_par:" + f.getEntryPoint());
                for (Function c : f.getCallingFunctions(monitor)) if (all.add(c)) roles.put(c, "appelant_de:" + f.getEntryPoint());
            }
        }
        for (Function f : all) {
            if (monitor.isCancelled()) break;
            File fo = new File(out, f.getEntryPoint().toString() + ".c");
            if (fo.exists()) continue;
            DecompileResults res = dec.decompileFunction(f, 120, monitor);
            String code = res.decompileCompleted() ? res.getDecompiledFunction().getC() : "// échec : " + res.getErrorMessage();
            StringBuilder hdr = new StringBuilder();
            hdr.append("// ").append(f.getName()).append(" @ ").append(f.getEntryPoint()).append("  ").append(roles.get(f)).append("\n// appelants :");
            for (Function c : f.getCallingFunctions(monitor)) hdr.append(' ').append(c.getEntryPoint());
            hdr.append("\n// appelés :");
            for (Function c : f.getCalledFunctions(monitor)) hdr.append(' ').append(c.getEntryPoint());
            hdr.append("\n");
            try (PrintWriter w = new PrintWriter(new OutputStreamWriter(new FileOutputStream(fo), "UTF-8"))) {
                w.print(hdr);
                w.print(code);
            }
            idx.println(f.getEntryPoint() + "\t" + f.getName() + "\t" + roles.get(f));
        }
        idx.close();
        dec.dispose();
    }
}
