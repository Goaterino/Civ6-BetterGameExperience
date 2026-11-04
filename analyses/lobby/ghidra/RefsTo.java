// Liste les fonctions qui référencent des adresses données (données ou code).
// Arguments : <fichier de sortie> <adresse1> [adresse2 ...]
//@category Civ6
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;
import java.io.*;

public class RefsTo extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] a = getScriptArgs();
        PrintWriter w = new PrintWriter(new OutputStreamWriter(new FileOutputStream(a[0], true), "UTF-8"));
        FunctionManager fm = currentProgram.getFunctionManager();
        for (int i = 1; i < a.length; i++) {
            Address t = toAddr(a[i]);
            for (Reference r : currentProgram.getReferenceManager().getReferencesTo(t)) {
                Function f = fm.getFunctionContaining(r.getFromAddress());
                w.println(t + "\t" + r.getFromAddress() + "\t" + r.getReferenceType() + "\t" + (f == null ? "-" : f.getEntryPoint() + "\t" + f.getName()));
            }
        }
        w.close();
    }
}
