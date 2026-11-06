function main(c)
{
    var ui = mappet.createUI().background();
    var scroll = ui.column(10, 0.1);

    scroll.getCurrent().scroll().rxy(0.5, 0.5).w(200).rh(0.8).anchor(0.5);
 
    var text = scroll.text("      Спасибо огромное этим людям!                                                                           " +
                                                       "[aСтроители:                 " +
                                                       "[cPe4eNyFkA                       " +
                                                       "[cTommy_Whinston                         [f7ankMan                         Secretar_NH                         ꜰʀᴇᴅᴅɪᴄᴋ                               NikBen                                 Dolonar                                 NullEntity                                 SyrexWL                                 KagyrPlay                            s0llydaren                            Гриззли                                                                                                               " +
                                                       "[bОзвучка:                   " +
                                                       "[fLonnye                                 Dolonar                                 Secretar_NH                                 [5Luna - youtube.com/@lunadub                                                                                   " +
                                                       "[eТех.Часть:           " +
                                                       "[cTommy_Whinston                      [cPe4eNyFkA                                 [5TheBendy - Очень помог со скриптами ꜰʀᴇᴅᴅɪᴄᴋ                                                                                                                                     " +
                                                       "[6Людям с дискорд сервера Nazzy!                                                            " +
                                                       "[fNazzy - Контент пак на оружия");
                                                       

    c.getSubject().openUI(ui);
}