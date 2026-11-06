function main(c)
{
  var random = Math.round(Math.random() * (9999 - 1000) + 1000);
  c.getServer().getStates().setString("password", random);
  c.executeCommand("/setblock -5 16 55 eclipsemod:doorcodeohrana 4")

      var random = Math.round(Math.random() * 10 - 1 + 1);
///////////////////////////////////////////////////////////////////////////////////////      
            
       //remove all parol
        c.executeCommand("/setblock -2 49 43 air");
        c.executeCommand("/setblock -30 54 77 air");
        c.executeCommand("/setblock 27 32 48 air");
        c.executeCommand("/setblock 26 18 90 air");
        c.executeCommand("/setblock 48 16 42 air");
        c.executeCommand("/setblock -28 16 67 air");
        c.executeCommand("/setblock -18 31 10 air");     
        c.executeCommand("/setblock 13 48 42 air");     
        c.executeCommand("/setblock -14 31 69 air");     
        c.executeCommand("/setblock 15 16 37 air");          
//////////////////////////////////////////////////////////////////////////////////////       
  
      if (random == 1)
      {
        c.executeCommand("/setblock -2 49 43 eclipsemod:parol");
      }
      if (random == 2)
      {
        c.executeCommand("/setblock -30 54 77 eclipsemod:parol");
      }
      if (random == 3)
      {
        c.executeCommand("/setblock 27 32 48 eclipsemod:parol")
      }
      if (random == 4)
      {
        c.executeCommand("/setblock 26 18 90 eclipsemod:parol")
      }
      if (random == 5)
      {
        c.executeCommand("/setblock 48 16 42 eclipsemod:parol")
      }
      if (random == 6)
      {
        c.executeCommand("/setblock -28 16 67 eclipsemod:parol")
      }
      if (random == 7)
      {
        c.executeCommand("/setblock -18 31 10 eclipsemod:parol")
      }
      if (random == 8)
      {
        c.executeCommand("/setblock 13 48 42 eclipsemod:parol")
      }
      if (random == 9)
      {
        c.executeCommand("/setblock -14 31 69 eclipsemod:parol")
      }
      if (random == 10)
      {
        c.executeCommand("/setblock 15 16 37 eclipsemod:parol")
      }
}       