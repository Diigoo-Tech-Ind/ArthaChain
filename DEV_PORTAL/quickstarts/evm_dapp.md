## Quickstart: dApp (EVM deploy & call)

1) Deploy contract via API:
```
curl -X POST "$API/contracts/deploy" \
  -H 'Content-Type: application/json' \
  -d '{"bytecode":"0x60006000556000600055"}'
```

2) Call contract function:
```
curl -X POST "$API/contracts/call" \
  -H 'Content-Type: application/json' \
  -d '{"to":"0x0000000000000000000000000000000000000000","data":"0x"}'
```

3) Using arthajs:
```
import { ArthaJS } from '../../sdk/arthajs/index'
```
