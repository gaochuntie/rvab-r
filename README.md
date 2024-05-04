# Android Dual Boot Utils
rvab command line multi call tool
manage <real> virtual A/B/C/D... slots for Android devices yes,these slots is not those slots
## Theory
double android userdata and other partitions and swaply use them to implement multi os coexistance.\
This tool provide a precise system to manage these partitions in order to simplify the process and enhance security.\
This system is similar to Android's a/b update mechanism, but it is a/b/c... in the true sense.\
So it comes the name <Real Virtual A/B Utils in Rust>
## Structure
The tool include 5 main part
1. Init : to generate template config file and do init for gpt partition table
2. Install : install(update) config as metadata on metadata area of each slot,they are areas 64mib before userdata partition
3. Switch : switch to another slot
4. List : list slots and their info
5. Archive : pack up a slot into a recovery flashable zip file,equal to a normal rom.zip
