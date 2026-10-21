# Builds a Scribe-style Greek transcript and its labels from a marked-up script.
# {F:..} filler to cut, {R:..} retake to cut, {O:..} optional, "|" a 0.8 s pause, "||" 1.6 s.
import json, re, sys

SCRIPT = """
{F:Εεε,} γεια σας παιδιά και καλώς ήρθατε σε ένα καινούριο βίντεο. | {F:Λοιπόν,} | σήμερα θα δούμε πώς
στήνουμε ένα project σε Python με το VS Code. Θα ξεκινήσουμε από τα βασικά και μετά θα πάμε σε πιο
προχωρημένα πράγματα. || Πρώτα απ' όλα, {R:ανοίγουμε το} | ανοίγουμε το terminal και γράφουμε python
version. Αν δεν ξέρετε ποια έκδοση έχετε, αυτή η εντολή σας τη δείχνει. | Αν δείτε τρία δώδεκα ή κάτι
πιο καινούριο, είστε εντάξει. || {F:Εντάξει,} τώρα θα φτιάξουμε ένα virtual environment. | {F:Βασικά,}
το virtual environment είναι, {F:ξέρεις,} ένας φάκελος που κρατάει τις βιβλιοθήκες του κάθε project
χωριστά. | Ας πούμε ότι έχουμε δύο projects που θέλουν διαφορετική έκδοση της ίδιας βιβλιοθήκης. |
Χωρίς venv θα είχαμε πρόβλημα. || Γράφουμε {O:λοιπόν} python μείον m venv venv και πατάμε enter. |
Μετά {R:το ενεργο-} | το ενεργοποιούμε με την εντολή activate. | Θα δείτε το όνομα του venv στην αρχή
της γραμμής, {F:εμ,} | εδώ αριστερά. || {O:Και} τώρα πάμε στο VS Code. | {R:Θα πατήσουμε στο} | Θα
κάνουμε κλικ στο Extensions και θα γράψουμε Python. | Εγκαθιστούμε την επέκταση της Microsoft. | Είναι
{O:πολύ} πολύ σημαντικό να είναι η επίσημη, γιατί οι άλλες, {F:ας πούμε,} δεν ενημερώνονται τόσο συχνά.
|| Ξέρεις τι κάνει αυτή η επέκταση; | Σου δίνει αυτόματη συμπλήρωση, debugging και πολλά άλλα. ||
{F:Λοιπόν,} | πάμε να φτιάξουμε το πρώτο μας αρχείο. | Πατάμε New File και το λέμε main dot py. | Μέσα
γράφουμε ένα απλό print. | {R:Και τρέχουμε} | Και το τρέχουμε με το κουμπί πάνω δεξιά. | Αν όλα είναι
εντάξει, θα δείτε το μήνυμα στο terminal. || {F:Εμ,} τώρα κάτι που πολλοί δεν ξέρουν. | Μπορείτε να
διαλέξετε ποιον interpreter χρησιμοποιεί το VS Code. | Πατάτε Control Shift P, γράφετε Select
Interpreter και διαλέγετε αυτόν από το venv. | Αυτό είναι, {F:δηλαδή,} το πιο συχνό λάθος που βλέπω. |
Ο κόσμος εγκαθιστά βιβλιοθήκες στο venv και μετά το VS Code τρέχει άλλη Python. || {F:Οκ,} | πάμε στο
τελευταίο κομμάτι, το αρχείο requirements. | {R:Το αρχείο αυτό} | Το requirements κρατάει όλες τις
βιβλιοθήκες που χρειάζεται το project. | Γράφουμε pip freeze και το αποθηκεύουμε. | Έτσι, όποιος
κατεβάσει το project σας, {F:ξέρετε,} μπορεί να τα εγκαταστήσει όλα με μία εντολή. || {F:Τέλος πάντων,}
| αυτά ήταν τα βασικά για σήμερα. | Δεν είναι κάτι δύσκολο, αλλά αν δεν το ξέρεις, χάνεις πολύ χρόνο. |
Στο επόμενο βίντεο θα δούμε, {F:ας πούμε,} πώς στήνουμε tests. | Αν σας άρεσε, κάντε like και
εγγραφείτε. | Τα λέμε!
"""

TYPES = {"F": "filler", "R": "retake", "O": "filler"}
tokens = re.findall(r"\{[FRO]:[^}]*\}|\|\||\||[^\s{|]+", SCRIPT)
words, gold, t = [], [], 500_000
for tok in tokens:
    if tok == "||":
        t += 1_600_000
        continue
    if tok == "|":
        t += 800_000
        continue
    mark = None
    texts = [tok]
    if tok.startswith("{"):
        mark, body = tok[1], tok[3:-1]
        texts = body.split()
    first = len(words)
    for text in texts:
        length = 120_000 + 45_000 * len(text)
        words.append({"id": f"w-{len(words)}", "text": text, "kind": "word",
                      "sourceStartUs": t, "sourceEndUs": t + length, "confidence": 0.98})
        t += length + 60_000
    if mark:
        gold.append({"from": first, "to": len(words) - 1, "type": TYPES[mark],
                     "note": " ".join(texts), "optional": mark == "O"})

out = sys.argv[1]  # the folder for el-synthetic.mic.json and el-a.json
transcript = {"schemaVersion": 1, "trackId": "el-synthetic.mic", "provider": "elevenlabs",
              "model": "scribe_v2", "language": "ell", "createdAt": "2026-10-21T00:00:00Z",
              "words": words}
json.dump(transcript, open(f"{out}/el-synthetic.mic.json", "w", encoding="utf-8"), ensure_ascii=False)
case = {"transcript": "el-synthetic.mic.json", "language": "el", "gold": gold}
json.dump(case, open(f"{out}/el-a.json", "w", encoding="utf-8"), ensure_ascii=False, indent=1)
print(len(words), "words,", sum(not g["optional"] for g in gold), "must,",
      sum(g["optional"] for g in gold), "optional")
