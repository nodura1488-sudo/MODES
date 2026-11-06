function main(c)
{
    var kd = c.getSubject().getStates().getNumber("kd");
    c.executeCommand("title @s actionbar "+kd);
}