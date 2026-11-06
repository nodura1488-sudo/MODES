function main(c)
{
    var alien = c.getSubject().getStates().getNumber("alien");
  
    if (alien == 1) 
    {
        var lay = c.getSubject().getStates().getNumber("lay");
        
        if (lay == 0)
        {
          c.executeCommand("/mp state add @s lay 1")
          var morph = mappet.createMorph("{Hitbox:{Eye:0.9f,Sneak:0.9f,Enabled:1b,Height:0.9f,Width:0.7f},Actions:{running:\"crouching\",idle:\"crouching_idle\"},Skin:\"c.s:AlienLay/skins/texture.png\",Settings:{Speed:0.07f},Name:\"chameleon.AlienLay\"}");
          c.getSubject().setMorph(morph);
        }
        if (lay == 1)
        {
          c.executeCommand("/mp state set @s lay 0")
          var morph = mappet.createMorph("{Hitbox:{Eye:0.9f,Sneak:1.8f,Enabled:1b,Height:2.5f},Skin:\"c.s:AlienStand/skins/texture.png\",Name:\"chameleon.AlienStand\"}");
          c.getSubject().setMorph(morph);
        }
    }
}