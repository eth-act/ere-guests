# Emits the guest pairs whose ELF differs between the base registry in $base and the head
# registry on stdin as a job matrix, with one entry per side. A pair the base registry lacks
# yields only a head entry.

def elfs:
  [ .stateless_validators[] | .name as $name | .artifacts[]
    | { key: "\($name)/\(.zkvm)",
        value: { stateless_validator: $name, zkvm, zkvm_version, elf_url, elf_sha256 } } ]
  | from_entries;

($base[0] | elfs) as $old
| { include:
      [ elfs | to_entries[] | .key as $pair | .value as $new
        | if $old[$pair] == null then $new + { side: "head" }
          elif $old[$pair].elf_sha256 != $new.elf_sha256 then
            ($old[$pair] + { side: "base" }), ($new + { side: "head" })
          else empty end ] }
