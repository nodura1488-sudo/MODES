function main(c)
{
        
            var ui = mappet.createUI(c, "passwordtest").background().closable(true);
            var textbox = ui.textbox().id("passwordstring").tooltip("Строка для кода");
            var label = ui.label("[fНужен [eКод[f:").id('labelpassword');
            var buttonok = ui.button("Открыть").id("buttonok");
            
            label.rxy(0.52,0.44).wh(100, 20).anchor(0.7);
            label.color(0x00ee22).background(0x88000000).labelAnchor(0.5);
            textbox.rxy(0.5, 0.5).wh(124, 20).anchor(0.5);
            buttonok.rxy(0.5, 0.55).wh(125, 20).anchor(0.5);
            c.getSubject().openUI(ui);
}

        
        function passwordtest(c)
        {
            var states = c.getServer().getStates();
            var uiContext = c.getSubject().getUIContext();
            var data = uiContext.getData();
            var pos = c.getSubject().getPosition();
            var zero = 0;
            var s = c.getSubject();
        
            if (uiContext.getLast() === "passwordstring")
                {
                    c.getWorld().playSound("eclipsemod:use_lose", pos.x, pos.y, pos.z, 1, 1.5);
                    s.swingArm();
                    var label = uiContext.get('labelpassword');
                    label.label("[fНужен [eКод[f:");
                    label.rxy(0.52,0.44).wh(100, 20).anchor(0.7);                    
                }
                
                    if (uiContext.getLast() === "buttonok")
                        {
                            var data = uiContext.getData();
                            var passtates = c.getServer().getStates();
                            
                            if (data.getString("passwordstring") == passtates.getString("password"))
                            {
                                s.swingArm();
                                c.executeCommand("/playsound eclipsemod:dopen master @a -5 16 55");
                                c.getWorld().setBlock(mappet.createBlockState("eclipsemod:dooropen", 3), -5, 16, 55);
                                c.getSubject().closeUI();
                            }
                            else
                            {
                                 var label = uiContext.get('labelpassword');
                                 label.label('[4Неверный [fКод[4:');
                                 label.rxy(0.52,0.44).wh(100, 20).anchor(0.7);
                                 c.getWorld().playSound("eclipsemod:door_faile", pos.x, pos.y, pos.z, 0.5, 1);
                                 s.swingArm();
                                 
                            }
                        }
        }