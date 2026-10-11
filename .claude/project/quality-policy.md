# Quality Policy

## Function length

A fix that clears the function-length gate must remove a repeated shape or give
a unit its own inputs and outputs. Reducing the count by compacting statements,
merging blocks, or moving the body behind an out-parameter without a new
responsibility is not a fix. Do not raise a baseline allowance to land a
feature; record why in the bead. A function may stay long only as a flat,
ordered checklist with no shared mutable state, named in the baseline with a
one-line reason in the commit message.
