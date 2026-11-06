function main(c)
{
    var ui = mappet.createUI(c, "handler").background();
    var players = c.getServer().getAllPlayers();
    var state = c.getServer().getStates().getNumber("goloca");
    var game = c.getServer().getStates().getNumber("game");
    var back = ui.graphics();
    
    if (game == 0)
    {
       var task = ui.label("Голосование за [aначало[f игры.");
    }
    else if (game == 1)
    {
       var task = ui.label("Голосование за [cконец[f игры."); 
    }
    
    var goloca = ui.label("Количество проголосовавших: [e"+state+"[f/"+players.size()+"").id("goloca");
    var yes = ui.button("Y/N").id("goloc");
    
    yes.rxy(0.482, 0.5).wh(40, 30);
    task.rxy(0.423, 0.406);
    back.rxy(0.372, 0.4).rect(0.372, 0.4, 250, 120, 0x88000000);
    back.rect(0.372, 0.4, 250, 15, 0xff009988);
    goloca.rxy(0.41, 0.455);
    
    c.getSubject().openUI(ui);
}

function handler(c)
{
    var s = c.getSubject();
    var uiContext = s.getUIContext();
    var players = c.getServer().getAllPlayers();
    
    if (uiContext.getLast() === "goloc" && s.getStates().getNumber("goloc") == 0)
    {
       c.getServer().getStates().add("goloca", 1);
       s.getStates().setNumber("goloc", 1);
       
       for (var i in players)
       {
          var context = players[i].getUIContext();

          if (!context) continue;
          context.get("goloca").label("Количество проголосовавших: [e" + c.getServer().getStates().getNumber("goloca") + "[f/" + players.size());
          context.sendToPlayer();
       }
    }
    else if (uiContext.getLast() === "goloc" && s.getStates().getNumber("goloc") == 1)
    {
       c.getServer().getStates().add("goloca", -1);
       s.getStates().setNumber("goloc", 0);
       
       for (var i in players)
       {
          var context = players[i].getUIContext();

          if (!context) continue;
          context.get("goloca").label("Количество проголосовавших: [e" + c.getServer().getStates().getNumber("goloca") + "[f/" + players.size());
          context.sendToPlayer();
       }
    }
    
    if (c.getServer().getStates().getNumber("goloca") >= c.getServer().getAllPlayers().size() && c.getServer().getStates().getNumber("game") == 0)
    {
       c.executeCommand("/mp event trigger @r start");
       c.getServer().getStates().setNumber("goloca", 0);
        
       var players = c.getServer().getAllPlayers();
       for (var i in players)
       {
           players[i].closeUI();
           players[i].getStates().setNumber("goloc", 0);
       }
    }
    else if (c.getServer().getStates().getNumber("goloca") >= c.getServer().getAllPlayers().size() && c.getServer().getStates().getNumber("game") == 1)
    {
       c.executeCommand("/mp event trigger @r gameover");
       c.getServer().getStates().setNumber("goloca", 0);
       
       var players = c.getServer().getAllPlayers();
       for (var i in players)
       {
           players[i].closeUI();
           players[i].getStates().setNumber("goloc", 0);
       }
    }
}