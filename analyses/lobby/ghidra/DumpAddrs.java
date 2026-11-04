// Décompile des fonctions à des adresses données, sans réanalyser.
// Arguments : <dossier de sortie> <adresse1> [adresse2 ...]  (une adresse peut être dans la fonction)
// Avec "+" devant une adresse, ajoute aussi ses appelés directs.
//@category Civ6
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.*;
import java.io.*;
import java.util.*;

public class DumpAddrs extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] a = getScriptArgs();
        File out = new File(a[0]);
        out.mkdirs();
        DecompInterface dec = new DecompInterface();
        dec.openProgram(currentProgram);
        FunctionManager fm = currentProgram.getFunctionManager();
        Set<Function> todo = new LinkedHashSet<>();
        for (int i = 1; i < a.length; i++) {
            boolean plus = a[i].startsWith("+");
            String s = plus ? a[i].substring(1) : a[i];
            Function f = fm.getFunctionContaining(toAddr(s));
            if (f == null) { println("pas de fonction en " + s); continue; }
            todo.add(f);
            if (plus) todo.addAll(f.getCalledFunctions(monitor));
        }
        for (Function f : todo) {
            DecompileResults res = dec.decompileFunction(f, 180, monitor);
            String code = res.decompileCompleted() ? res.getDecompiledFunction().getC() : "// échec : " + res.getErrorMessage();
            StringBuilder hdr = new StringBuilder("// " + f.getName() + " @ " + f.getEntryPoint() + "\n// appelants :");
            for (Function c : f.getCallingFunctions(monitor)) hdr.append(' ').append(c.getEntryPoint());
            hdr.append("\n// appelés :");
            for (Function c : f.getCalledFunctions(monitor)) hdr.append(' ').append(c.getEntryPoint());
            hdr.append("\n");
            try (PrintWriter w = new PrintWriter(new OutputStreamWriter(new FileOutputStream(new File(out, f.getEntryPoint() + ".c")), "UTF-8"))) {
                w.print(hdr);
                w.print(code);
            }
        }
        dec.dispose();
    }
}
