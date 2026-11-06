function main(c)
{  
     //removeallsafe
      c.executeCommand("/setblock 10 31 24 air");
      c.executeCommand("/setblock -4 15 48 air");
      c.executeCommand("/setblock -7 31 69 air");
      c.executeCommand("/setblock -18 31 24 air");
      c.executeCommand("/setblock -2 48 41 air");
      c.executeCommand("/setblock -7 15 52 air");
      c.executeCommand("/setblock 54 15 63 air");
      c.executeCommand("/setblock 28 31 50 air");
      c.executeCommand("/setblock -18 54 46 air");
      c.executeCommand("/setblock -12 15 56 air");
      c.executeCommand("/setblock -27 48 61 air");
      c.executeCommand("/setblock -6 17 71 air");
      c.executeCommand("/setblock 29 17 64 air");
      c.executeCommand("/setblock 26 48 61 air");
      c.executeCommand("/setblock -6 48 71 air");
      c.executeCommand("/setblock -26 54 46 air");
      c.executeCommand("/setblock -6 17 95 air");
      c.executeCommand("/setblock -19 48 41 air");
      c.executeCommand("/setblock 24 48 42 air");
      c.executeCommand("/setblock -30 15 55 air");
    
         var random = Math.round(Math.random() * 3 - 1 + 1);
                      
         if (random == 1)
         {
           c.executeCommand("/setblock 10 31 24 eclipsemod:safearmor 3");
           c.executeCommand("/setblock -4 15 48 eclipsemod:safearmor 5");
           c.executeCommand("/setblock -7 31 69 eclipsemod:safearmor 4");
           c.executeCommand("/setblock -18 54 46 eclipsemod:safeguns 4");
           c.executeCommand("/setblock -27 48 61 eclipsemod:safeguns 3");
           c.executeCommand("/setblock -6 17 71 eclipsemod:safeguns 4");
           c.executeCommand("/setblock -19 48 41 eclipsemod:safeshotguns 4");
         }
         if (random == 2)
         {
           c.executeCommand("/setblock -18 31 24 eclipsemod:safearmor 3");
           c.executeCommand("/setblock -2 48 41 eclipsemod:safearmor 3");
           c.executeCommand("/setblock -7 15 52 eclipsemod:safearmor 3");
           c.executeCommand("/setblock 29 17 64 eclipsemod:safeguns 3");
           c.executeCommand("/setblock 26 48 61 eclipsemod:safeguns 4");
           c.executeCommand("/setblock -12 15 56 eclipsemod:safeguns 5");
           c.executeCommand("/setblock 24 48 42 eclipsemod:safeshotguns 4");
         } 
         if (random == 3)
         {
           c.executeCommand("/setblock 54 15 63 eclipsemod:safearmor 5");
           c.executeCommand("/setblock 28 31 50 eclipsemod:safearmor ");
           c.executeCommand("/setblock -18 54 46 eclipsemod:safearmor 4");
           c.executeCommand("/setblock -6 48 71 eclipsemod:safeguns");
           c.executeCommand("/setblock -26 54 46 eclipsemod:safeguns");
           c.executeCommand("/setblock -6 17 95 eclipsemod:safeguns");
           c.executeCommand("/setblock -30 15 55 eclipsemod:safeshotguns 3");
         } 
}