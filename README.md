# Android Dual Boot / Multi-BOOT Utils 2024
rvab command line multi call tool is designed for android multi-boot next generation.I have given a new idea on it , real virtual a/b.You can own multi os on your android phone ,and manage them like a/b update do.
What's more?You can even hotly sideload a rom from an online server,for example,a ftp server (This function need your gpt partitions fully doubled except xbl,so it's dangerous,but once anyone succeed on a device, it means this device is fully supported).
manage <real> virtual A/B/C/D... slots for Android devices, yes,these slots is not those slots of google's
# Bad News
This project may be in a very very slow development status.Because I need to do lots of things both on this project and my life.
# Good News
This project relies nothing on google's changes of android device,rely nothing on device band,rely nothing on hardware arch.
It only relies on the stability of rust gpt lib.
SO!!!!Whatever you android version is, your sys is 64bit or 32bit,your cpu is arm or x86.This project will never be out of date.
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
