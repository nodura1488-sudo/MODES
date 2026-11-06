function main(c)
{
    {
      var random = Math.round(Math.random() * 4 - 1 + 1);
///////////////////////////////////////////////////////////////////////////////////////      
            
       //remove all shitok
        c.getWorld().setBlock(mappet.createBlockState("minecraft:air", 2), -26, 49, 45);
        c.getWorld().setBlock(mappet.createBlockState("minecraft:air", 2), 4, 49, 70);
        c.getWorld().setBlock(mappet.createBlockState("minecraft:air", 2), -23, 32, 46);
        c.getWorld().setBlock(mappet.createBlockState("minecraft:air", 2), 27, 16, 55);
        
//////////////////////////////////////////////////////////////////////////////////////              
      if (random == 1)
      {
        c.executeCommand("/setblock -26 49 45 eclipsemod:usefuse0");
      }
      if (random == 2)
      {
        c.executeCommand("/setblock 4 49 70 eclipsemod:usefuse0 5");
      }
      if (random == 3)
      {
        c.executeCommand("/setblock -23 32 46 eclipsemod:usefuse0 4")
      }
      if (random == 4)
      {
        c.executeCommand("/setblock 27 16 55 eclipsemod:usefuse0 5")
      }
    }
}