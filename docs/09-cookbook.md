# Cookbook

Complete, paste-ready examples. Each one exercises a different corner of the
language.

## 1. CI/CD pipeline (decisions, retry loops, groups)

```gfd
dir: TB
default node [rounded]

node commit "Push to main" [stadium, fill=#e8f5e9]
node build  "Build + unit tests"
gate "All green?" [diamond, fill=#fff3e0]
node fix "Fix and re-push" [fill=#ffebee] right-of gate gap 90

commit -> build -> gate
gate -> fix : "no" [dashed]
fix ..> commit : "retry"

group deploy "Deploy" {
  dir: LR
  node stage "Staging"
  node smoke "Smoke tests"
  node prod  "Production" [bold]
  stage -> smoke -> prod
}
gate -> stage : "yes"
prod -> announce
node announce "Announce in Slack" [ellipse]
```

## 2. Org chart (classes, ports, multi-line labels)

```gfd
use people
dir: TB

ceo  = Person("Robin", "CEO")
cto  = Person("Sam", "CTO")
cfo  = Person("Alex", "CFO")
eng1 = Person("Kai", "Platform lead")
eng2 = Person("Noor", "Product lead")

ceo.feet -> cto.head
ceo.feet -> cfo.head
cto.feet -> eng1.head
cto.feet -> eng2.head
```

## 3. Microservice map (starter aws library, chains, defaults)

```gfd
use aws
dir: LR
default edge [stroke=#8a94a6]

gw    = Gateway("API GW")
authf = Lambda("authorize")
orders = Service("Orders", #e3f2fd)
q     = Queue("order-events")
work  = Lambda("fulfill")
db    = Db("orders-db")
files = Bucket("invoices")

gw.priv -> authf.trigger
authf.out -> orders.in
orders.out -> q.in
q.out -> work.trigger
work.out -> db.write
work.out -> files : "PDF" [dashed]
orders.out -> db.write
```

## 4. State machine (circles, reversed arrows, precise placement)

```gfd
dir: LR
default node [circle, fill=#ede7f6]

idle    "Idle"    @ (0, 0)
running "Running" @ (220, 0)
paused  "Paused"  @ (220, 160)
done    "Done"    [stadium, fill=#e8f5e9] @ (440, 0)

idle -> running : "start"
running -> paused : "pause"
running <- paused : "resume"
running -> done : "finish"
done ..> idle : "reset"
```

## 5. Legend + unconnected annotations (relative placement)

```gfd
main "The actual diagram" [rounded] @ (0, 0)

// A legend floats nearby with no edges at all:
legend "Legend" [card, fill=#fffde7] right-of main gap 140
l1 "solid = sync call"   [rect, w=170] below legend gap 16
l2 "dashed = async"      [rect, w=170, dashed] below l1 gap 12
```

## 6. Themed diagram with a shared palette (variables + bundles)

```gfd
// palette
ink    = #263238
mist   = #eceff1
sky    = #e1f0fa
grass  = #e8f5e9

// styles
box    = [rounded, fill=$mist, text=$ink]
input  = [$box, parallelogram, fill=$sky]
happy  = [$box, fill=$grass]

req$input  "HTTP request"
parse$box  "Parse + validate"
ok$happy   "200 OK" 

req -> parse -> ok
```

## 7. Project-local library (portable repos)

`docs/house-style.gfd`:
```gfd
brand = #5c6bc0
class Step(t) { shape = [rounded]; label = $t; fill = #e8eaf6; stroke = $brand }
```

`docs/flow.gfd`:
```gfd
use "./house-style.gfd"
a = Step("Ingest")
b = Step("Transform")
c = Step("Load")
a -> b -> c
```

Clone the repo anywhere — the import travels with it.
