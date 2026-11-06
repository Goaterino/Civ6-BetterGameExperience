// Décompile toutes les fonctions dont l'entrée est dans [début, fin). Arguments : <dossier> <début> <fin>
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.*;
import java.io.*;

public class DumpRange extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] a = getScriptArgs();
        File out = new File(a[0]);
        out.mkdirs();
        DecompInterface d = new DecompInterface();
        d.openProgram(currentProgram);
        FunctionIterator it = currentProgram.getFunctionManager().getFunctions(toAddr(a[1]), true);
        long end = Long.parseLong(a[2], 16);
        while (it.hasNext()) {
            Function f = it.next();
            if (f.getEntryPoint().getOffset() >= end) break;
            DecompileResults res = d.decompileFunction(f, 120, monitor);
            try (PrintWriter w = new PrintWriter(new File(out, f.getEntryPoint() + ".c"), "UTF-8")) {
                w.println(res.decompileCompleted() ? res.getDecompiledFunction().getC() : "// échec");
            }
        }
    }
}
