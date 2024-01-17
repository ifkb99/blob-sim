# Blob Sim
A relatively simple bacteria-inspired sim

## What is this?

In short:

- Differently coloured blobs correspond to different genetic codes
- Blobs spawn when there are less than `N` on screen, or when one decides to reproduce. Spawned blobs have random genomes, while children copy their parents, with the chance for mutation
- Genomes create the "brain" of a blob, which is a simple neural net. The few inputs (energy, time, nearby chemicals, etc) allow it to determine where to move, and how quickly to do so
- Blobs can detect chemicals emitted by food. Naturally they evolve to approach the food, or find the optimal strategy to collect food (often moving diagonally to wrap around the screen and cover everything)
