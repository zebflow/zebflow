## Milestone 1

- Comment Auth for now so can directly create pipelines
- Focus on RESTful API pipeline first
- Create 3 webhook
    - GET /articles -> n.script , just dummy but get real ctx data
    - POST /articles -> n.script , just dummy posted but get real ctx data
    - GET /articles/{slug} -> n.script , just dummy posted but get real ctx data
- Make sure, multiple path on /articles work GET / POST with separate process of course 
- Make sure the dynamic param /articles/{slug}

If see the current system doesn't accomodate it yet,
then its your responsibility to halt your current task,
fix the foundation first, because we are in the phase of
building the platform

every anomaly = need to cleverly fix foundational error 

you can's say it can't. you test so you improve

-----------------------------
Update for Milestone 1 - 1
- input should can be traversed by n.script node
- all node definition should share same interface
    - and they need to precisely show the description of the node and input output from the very node description
    - including the detailed rustdoc
    - update node structure so it can store like i said before
        - description that will automatically be used by ui
        - input output format
        - is available for node script or not, for ex. n.pg.query, should be able from n.script 
        - is available n registered as AI tool or not, if yes, define the tool here too in node description
- its very important as we talk this before, n.script is use sandboxed deno from language mod, and it works by injecting it with external script, so deno can access the variable, and also can access certain nodes that is n.script available

-----------------------------
Update for Milestone 1 - 2
- create pipeline of webhook - script - web.render
    - of course you also need to create template to do this


=============================

## Milestone 2
- Create js library for visualizing and editing pipeline graph, lets say zeb/graphui
- Update the pipeline edit page, right after selecting pipeline
- In pipeline editor use the example of zeb/graphui and render the pipeline json into that. The layout in pipeline editor is categorical icon at left, if pressed show context menu
- optimize platform zeb/related-libraries. for example if you need some functionalities that possibly needed also by other, make it into the zeb/ the main 3 shareable library like shareutil and interact .. i think it should be 3 accourding the md
- then create pipeline creation ui
    - Create new button
    - Open dialog that choose trigger, give name, select folder / new folder
    - Submit, go to editor 
    - In the editor in every node there is edit button
    - On edit shown dialog, to change things
    - For main native / basic node, the dialog node edit is designed manually
    - Submit
- make sure the pipeline registered
- implement and update docs
- Also implement and update docs regarding the draft and production smart differentiation.. like if its on producition it has hash, then if changed it show. i think we already done this. just check it
- Implement hits info either in memory or somewhere.. hit success and hit failed , count of it. and N latest error for each pipeline

-----------------------------
Update for Milestone 2 - 2

## Milestone 3 
- Implement the js mechanism like you want to do right now
project-wide library manifest + symbol-to-chunk map,
save-time incremental compile coordinator,
shared chunk emission + page chunk emission + route manifest linking.