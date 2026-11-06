function main(c)
{
    var ui = mappet.createUI().background();
    var password = c.getServer().getStates().getString("password");
    var scroll = ui.column(10, 10);

    scroll.getCurrent().scroll().rxy(0.5, 0.5).w(200).rh(0.2).anchor(0.4);
 
    var text = scroll.text("[4Никто не должен знать!                                                                         " +
                                                       "[f"+password+"");

    c.getSubject().openUI(ui);
}